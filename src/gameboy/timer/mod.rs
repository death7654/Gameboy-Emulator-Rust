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
        if ff04 == 0
        {
            self.div_counter = 0;
        }
        self.div_counter = self.div_counter.wrapping_add(osc_cycles);
        
        while self.div_counter >= 256 {
            // Read current DIV value (at RAM address 0xFF04)
            let ff04 = self.ram.borrow().read(0xFF04);
            // Update DIV (typically a simple increment, wrapping at 0xFF)
            self.ram
                .borrow_mut()
                .update_div(ff04.wrapping_add(1));
            self.div_counter -= 256;
        }

        // -------------------
        // Handle any pending timer overflow delays.
        // According to hardware, when TIMA overflows, the reload happens after 4 clocks
        // (i.e., one T-cycle or 4 oscillator cycles). During that one cycle, TIMA reads as 0.
        // -------------------
        if let Some(ref mut delay) = self.timer_overflow_delay {
            if osc_cycles >= *delay {
                // The delay expires during this tick.
                *delay = 0;
                // Reload TIMA with TMA.
                let tma = self.ram.borrow().read(0xFF06);
                self.ram.borrow_mut().write(0xFF05, tma);
                // Set the Timer Interrupt flag (bit 2) in IF (address 0xFF0F).
                let if_reg = self.ram.borrow().read(0xFF0F) | 0b0000_0100;
                self.ram.borrow_mut().write(0xFF0F, if_reg);
                // Clear the pending overflow delay.
                self.timer_overflow_delay = None;
            } else {
                // Otherwise, just decrement the pending delay.
                *delay -= osc_cycles;
            }
        }

        // -------------------
        // Timer (TIMA) update.
        // -------------------

        // Read TAC register (0xFF07)
        let ff07 = self.ram.borrow().read(0xFF07);
        // Timer is enabled if bit 2 of TAC is set.
        if ff07 & 0b0000_0100 != 0 {
            // Determine the frequency based on the lower two bits of TAC.
            // These values are in oscillator cycles.
            let frequency: u16 = match ff07 & 0b0000_0011 {
                0 => 1024, // 4096 Hz: 1024 osc cycles per increment.
                1 => 16,   // 262144 Hz: 16 osc cycles.
                2 => 64,   // 65536 Hz: 64 osc cycles.
                3 => 256,  // 16384 Hz: 256 osc cycles.
                _ => unreachable!(),
            };

            self.tima_counter += osc_cycles;
            // As long as we have accumulated enough cycles,
            // update TIMA (located at 0xFF05).
            while self.tima_counter >= frequency {
                self.tima_counter -= frequency;
                // Read current TIMA value
                let ff05 = self.ram.borrow().read(0xFF05);
                // Increment TIMA. Using overflowing_add—to check for overflow.
                let (new_val, overflowed) = ff05.overflowing_add(1);
                if !overflowed {
                    self.ram.borrow_mut().write(0xFF05, new_val);
                } else {
                    // TIMA overflow from 0xFF to 0x00 occurs.
                    // The hardware resets TIMA to TMA after a delay.
                    // Until the reload, TIMA reads as zero.
                    self.ram.borrow_mut().write(0xFF05, 0);
                    // Schedule the delayed reload.
                    // (Delay is one T-cycle = 4 oscillator clocks.)
                    self.timer_overflow_delay = Some(4);
                }
            }
        }
    }

}
