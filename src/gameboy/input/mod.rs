use sdl2::keyboard::Keycode;


pub struct Joypad {
    dpad: u8,
    buttons: u8,
    reg: u8,
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            dpad: 0x0F,
            buttons: 0x0F,
            reg: 0xFF, 
        }
    }

    pub fn set_key(&mut self, key: Keycode, pressed: bool) {
        // For the D-Pad, we use WASD (W: Up, A: Left, S: Down, D: Right)
        match key {
            Keycode::W => { // Up corresponds to bit 2
                if pressed { self.dpad &= 0b11111011; } else { self.dpad |= 0b00000100; }
            },
            Keycode::A => { // Left corresponds to bit 1
                if pressed { self.dpad &= 0b11111101; } else { self.dpad |= 0b00000010; }
            },
            Keycode::S => { // Down corresponds to bit 3
                if pressed { self.dpad &= 0b11110111; } else { self.dpad |= 0b00001000; }
            },
            Keycode::D => { // Right corresponds to bit 0
                if pressed { self.dpad &= 0b11111110; } else { self.dpad |= 0b00000001; }
            },
            // For the Button group, we use different keys:
            Keycode::Z => { // A button is bit 0
                if pressed { self.buttons &= 0b11111110; } else { self.buttons |= 0b00000001; }
            },
            Keycode::X => { // B button is bit 1
                if pressed { self.buttons &= 0b11111101; } else { self.buttons |= 0b00000010; }
            },
            Keycode::Backspace => { // Select button is bit 2
                if pressed { self.buttons &= 0b11111011; } else { self.buttons |= 0b00000100; }
            },
            Keycode::Return => { // Start button is bit 3
                if pressed { self.buttons &= 0b11110111; } else { self.buttons |= 0b00001000; }
            },
            _ => {}
        }
    }


    pub fn read(&self) -> u8 {
        // Check selection bits (bits 4 and 5)
        let select_dpad = (self.reg & 0x10) == 0;
        let select_buttons = (self.reg & 0x20) == 0;

        let lower = match (select_dpad, select_buttons) {
            (true, false) => self.dpad,
            (false, true) => self.buttons,
            (true, true)  => self.dpad & self.buttons, // both groups selected: bitwise AND
            (false, false) => 0x0F, // if neither group is selected, return all 1s
        };

        (self.reg & 0xF0) | lower
    }

    pub fn write(&mut self, value: u8) {
        // Only bits 4-7 are writable; keep the lower nibble as is.
        self.reg = (self.reg & 0x0F) | (value & 0xF0);
    }
}
