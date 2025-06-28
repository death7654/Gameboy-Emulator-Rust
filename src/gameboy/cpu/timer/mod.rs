/*
The timer has 4 main registers

    0xFF04
        - stores the upper 8 bits of the div counter
        - called the DIV
        - this is represented by the div_counter in the timer structure
    0xFF05
        - TIMA or the timer counter
        - this is represented by the tima_counter
    0xFF06
        - the value found in this location is stored in the TIMA when it overflows
        - called the Timer Modulo
    0xFF07
        - Known as the TAC or the timer control
        - bit 2 is used to check if the TIMA counter should count
        - bits 0 and 1 are used to set the frequency
*/

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
        // value of div counter after boot rom is loaded and offloaded
        Timer {
            div_counter: 0xABCC,
            tima_counter: 0,
            timer_overflow_delay: false,
            tima_overflowed: false,
            ram,
        }
    }
    pub fn timer(&mut self, t_cycles: u16) {
        // t_cycles are constant, the value is always 4
        let mut ram = self.ram.borrow_mut();

        if ram.div_written {
            self.div_counter = 0;
            ram.div_written = false;
        }
        self.div_counter = self.div_counter.wrapping_add(t_cycles);

        ram.update_div((self.div_counter >> 8) as u8);

        // the overflowed value is set after one call of the timer function;

        if self.timer_overflow_delay {
            self.timer_overflow_delay = false;
        } else if self.tima_overflowed {
            // writes the value found in the timer modulo into the timer counter
            let timer_modulo = ram.read(0xFF06);
            ram.write(0xFF05, timer_modulo);
            self.tima_counter = timer_modulo as u16; // sets the value in our local counter as well

            // writes an interrupt
            let interrupt = (ram.read(0xFF0F)) | 0b0000_0100;
            ram.write(0xFF0F, interrupt);

            self.tima_overflowed = false;
        }

        let tac = ram.read(0xFF07);
        if tac & 0x4 != 0 {
            let frequency: u16 = match tac & 0b0000_0011 {
                0 => 1024, // 4096 Hz: 1024 t-cycles cycles per increment.
                1 => 16,   // 262144 Hz: 16 t-cycles cycles.
                2 => 64,   // 65536 Hz: 64 t-cycles cycles.
                3 => 256,  // 16384 Hz: 256 t-cycles cycles.
                _ => unreachable!(),
            };

            self.tima_counter += t_cycles;

            //increments the tima counter
            while self.tima_counter >= frequency {
                self.tima_counter -= frequency;

                let ff05 = ram.read(0xFF05);
                let (new_val, overflowed) = ff05.overflowing_add(1);

                // overflowed operations
                if overflowed {
                    self.timer_overflow_delay = true;
                    self.tima_overflowed = true;
                } else {
                    ram.write(0xFF05, new_val);
                }
            }
        }
    }
}
