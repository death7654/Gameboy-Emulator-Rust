use std::cell::RefCell;
use std::rc::Rc;

use crate::gameboy::ram::RAM;

pub struct Timer {
    div_counter: u16,
    tima_counter: u16,

    timer_overflow_delay: Option<u16>,
    ram: Rc<RefCell<RAM>>,
}

impl Timer {
    pub fn new(ram: Rc<RefCell<RAM>>) -> Self {
        Timer {
            div_counter: 0,
            tima_counter: 0,
            timer_overflow_delay: None,
            ram,
        }
    }
    pub fn timer(&mut self, osc_cycles: u16) {
        let ff04 = self.ram.borrow().read(0xFF04);
        if ff04 == 0 {
            self.div_counter = 0;
        }
        self.div_counter = self.div_counter.wrapping_add(osc_cycles);

        while self.div_counter >= 256 {
            // Read current DIV value (at RAM address 0xFF04)
            // Update DIV (typically a simple increment, wrapping at 0xFF)
            self.ram.borrow_mut().update_div(ff04.wrapping_add(1));
            self.div_counter -= 256;
        }

        if let Some(ref mut delay) = self.timer_overflow_delay {
            if osc_cycles >= *delay {
                *delay = 0;
                let tma = self.ram.borrow().read(0xFF06);
                self.ram.borrow_mut().write(0xFF05, tma);
                let if_reg = self.ram.borrow().read(0xFF0F) | 0b0000_0100;
                self.ram.borrow_mut().write(0xFF0F, if_reg);
                self.timer_overflow_delay = None;
            } else {
                *delay -= osc_cycles;
            }
        }

        let ff07 = self.ram.borrow().read(0xFF07);
        if ff07 & 0b0000_0100 != 0 {
            let frequency: u16 = match ff07 & 0b0000_0011 {
                0 => 1024, // 4096 Hz: 1024 osc cycles per increment.
                1 => 16,   // 262144 Hz: 16 osc cycles.
                2 => 64,   // 65536 Hz: 64 osc cycles.
                3 => 256,  // 16384 Hz: 256 osc cycles.
                _ => unreachable!(),
            };

            self.tima_counter += osc_cycles;
            while self.tima_counter >= frequency {
                self.tima_counter -= frequency;
                let ff05 = self.ram.borrow().read(0xFF05);
                let (new_val, overflowed) = ff05.overflowing_add(1);
                if !overflowed {
                    self.ram.borrow_mut().write(0xFF05, new_val);
                } else {
                    self.ram.borrow_mut().write(0xFF05, 0);
                    self.timer_overflow_delay = Some(4);
                }
            }
        }
    }
}
