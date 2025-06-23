use std::cell::RefCell;
use std::rc::Rc;

use crate::gameboy::ram::RAM;

pub struct Timer {
    div_counter: u16,
    tima_counter: u16,

    timer_overflow_delay: bool,
    pub tima_overflowed: bool,
    ram: Rc<RefCell<RAM>>,
}

impl Timer {
    pub fn new(ram: Rc<RefCell<RAM>>) -> Self {
        Timer {
            div_counter: 0xABCC,
            tima_counter: 0,
            timer_overflow_delay: false,
            tima_overflowed: false,
            ram,
        }
    }
    pub fn timer(&mut self, mut osc_cycles: u16) {
        osc_cycles = osc_cycles/4;
        if self.ram.borrow().div_written {
            self.div_counter = 0;
            self.ram.borrow_mut().div_written = false;
        }

        self.div_counter = self.div_counter.wrapping_add(osc_cycles);

        self.ram
            .borrow_mut()
            .update_div((self.div_counter >> 8) as u8);

        if self.timer_overflow_delay {
            self.timer_overflow_delay = false;
        } else if self.tima_overflowed {
            let ff06 = self.ram.borrow().read(0xFF06);
            self.ram.borrow_mut().write(0xFF05, ff06);
            self.tima_counter = ff06 as u16;
            let mut interrupt = self.ram.borrow().read(0xFF0F);
            interrupt |= 0b0000_0100;
            self.ram.borrow_mut().write(0xFF0F, interrupt);
        }

        let ff07 = self.ram.borrow().read(0xFF07);
        if ff07 & 0x4 != 0 {
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
                    self.timer_overflow_delay = true;
                    self.tima_overflowed = true;
                }
            }
        }
    }
}
