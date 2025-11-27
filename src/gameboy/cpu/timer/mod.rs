use crate::gameboy::mmu::MMU;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Timer {
    div_counter: u16,
    timer_overflow_delay: bool,
    pub tima_overflowed: bool,
    ram: Rc<RefCell<MMU>>,
}

impl Timer {
    pub fn new(ram: Rc<RefCell<MMU>>) -> Self {
        // DIV starts at 0xAB after boot, so div_counter should represent this
        Timer {
            div_counter: 0xAB00, // Upper 8 bits = 0xAB
            timer_overflow_delay: false,
            tima_overflowed: false,
            ram,
        }
    }

    pub fn timer(&mut self, t_cycles: u16) {
        let mut ram = self.ram.borrow_mut();
        let tac = ram.read(0xFF07);
        let timer_enabled = tac & 0x4 != 0;

        // Handle DIV write (reset)
        if ram.div_written {
            let old_bit = self.get_timer_bit(self.div_counter, tac);
            self.div_counter = 0;
            ram.div_written = false;

            // DIV reset can cause a falling edge
            let new_bit = self.get_timer_bit(self.div_counter, tac);
            if timer_enabled && old_bit && !new_bit {
                let tima = ram.read(0xFF05);
                let (new_val, overflowed) = tima.overflowing_add(1);

                if overflowed {
                    ram.write(0xFF05, 0x00);
                    self.timer_overflow_delay = true;
                    self.tima_overflowed = true;
                } else {
                    ram.write(0xFF05, new_val);
                }
            }
        }

        // Process each t-cycle individually to catch all edges
        for _ in 0..t_cycles {
            // Handle overflow state machine FIRST
            if self.timer_overflow_delay {
                self.timer_overflow_delay = false;
                // tima_overflowed stays true
            } else if self.tima_overflowed {
                let timer_modulo = ram.read(0xFF06);
                ram.write(0xFF05, timer_modulo);

                let interrupt = ram.read(0xFF0F) | 0b0000_0100;
                ram.write(0xFF0F, interrupt);

                self.tima_overflowed = false;
            }

            // Store old bit state
            let old_bit = if timer_enabled {
                self.get_timer_bit(self.div_counter, tac)
            } else {
                false
            };

            // Increment DIV by 1 t-cycle
            self.div_counter = self.div_counter.wrapping_add(1);

            // Update DIV register (upper 8 bits)
            ram.update_div((self.div_counter >> 8) as u8);

            // Check for falling edge
            if timer_enabled {
                let new_bit = self.get_timer_bit(self.div_counter, tac);

                if old_bit && !new_bit {
                    let tima = ram.read(0xFF05);
                    let (new_val, overflowed) = tima.overflowing_add(1);

                    if overflowed {
                        ram.write(0xFF05, 0x00);
                        self.timer_overflow_delay = true;
                        self.tima_overflowed = true;
                    } else {
                        ram.write(0xFF05, new_val);
                    }
                }
            }
        }
    }
    // Gets the state of the bit used for timer incrementing
    fn get_timer_bit(&self, counter: u16, tac: u8) -> bool {
        let bit_position = match tac & 0b11 {
            0 => 9, // 4096 Hz: bit 9 (every 1024 t-cycles)
            1 => 3, // 262144 Hz: bit 3 (every 16 t-cycles)
            2 => 5, // 65536 Hz: bit 5 (every 64 t-cycles)
            3 => 7, // 16384 Hz: bit 7 (every 256 t-cycles)
            _ => unreachable!(),
        };
        (counter >> bit_position) & 1 == 1
    }
}
