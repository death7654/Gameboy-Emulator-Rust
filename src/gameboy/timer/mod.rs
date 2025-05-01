use std::cell::RefCell;
use std::rc::Rc;

use crate::gameboy::ram::RAM;

pub struct Timer {
    div_counter: u16,
    tima_counter: u16,
    ram: Rc<RefCell<RAM>>,
}

impl Timer {
    pub fn new(ram: Rc<RefCell<RAM>>) -> Self {
        Timer {
            div_counter: 0,
            tima_counter: 0,
            ram,
        }
    }
    pub fn timer(&mut self, cycles: u16) {
        let ff04 = self.ram.borrow().read(0xFF04);
        self.div_counter = self.div_counter.wrapping_add(cycles);

        if self.div_counter % 256 == 0 {
            self.ram.borrow_mut().update_div(ff04.wrapping_add(1));
            if self.div_counter >= 256 {
                self.div_counter -= 256;
            }
        }

        let ff07 = self.ram.borrow().read(0xFF07);
        if ff07 & 0b0000_0100 != 0 {
            // Enable timer if bit 2 is set
            let frequency: u16 = match ff07 & 0b0000_0011 {
                0 => 1024,
                1 => 16,
                2 => 64,
                3 => 256,
                _ => unreachable!(),
            };

            self.tima_counter += cycles as u16;
            let ff05 = self.ram.borrow().read(0xFF05);
            let tima_result;
            let mut tima_overflow = false;

            // Handle TIMA overflow based on TMA behavior
            if self.tima_counter >= frequency {
                self.tima_counter -= frequency;
                (tima_result, tima_overflow) = ff05.overflowing_add(1);
                self.ram.borrow_mut().write(0xFF05, tima_result);
            }

            // Interrupt logic and TMA synchronization
            if tima_overflow {
                let reset_value = self.ram.borrow().read(0xFF06); // TMA
                self.ram.borrow_mut().write(0xFF05, reset_value);

                let interrupt_flag = self.ram.borrow().read(0xFF0F) | 0b0000_0100;
                self.ram.borrow_mut().write(0xFF0F, interrupt_flag);
            }
        }
    }
}
