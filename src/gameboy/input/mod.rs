use sdl2::keyboard::Keycode;

#[derive(Clone)]
pub struct Joypad {
    dpad: u8,
    buttons: u8,
    reg: u8,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            dpad: 0x0F,
            buttons: 0x0F,
            reg: 0xFF,
        }
    }

    // update key state based on keydown/keyup
    pub fn set_key(&mut self, key: Keycode, pressed: bool) {
        match key {
            Keycode::W => {
                // bit 2 or the up key
                if pressed {
                    self.dpad &= !0x04;
                } else {
                    self.dpad |= 0x04;
                }
            }
            Keycode::A => {
                // bit 1 or the left key
                if pressed {
                    self.dpad &= !0x02;
                } else {
                    self.dpad |= 0x02;
                }
            }
            Keycode::S => {
                // bit 3 or the down key
                if pressed {
                    self.dpad &= !0x08;
                } else {
                    self.dpad |= 0x08;
                }
            }
            Keycode::D => {
                // bit 0 or the right key
                if pressed {
                    self.dpad &= !0x01;
                } else {
                    self.dpad |= 0x01;
                }
            }
            Keycode::Z => {
                // bit 1 or the A key
                if pressed {
                    self.buttons &= !0x01;
                } else {
                    self.buttons |= 0x01;
                }
            }
            Keycode::X => {
                // bit 2 or the B key
                if pressed {
                    self.buttons &= !0x02;
                } else {
                    self.buttons |= 0x02;
                }
            }
            Keycode::Backspace => {
                // bit 2 or the select button
                if pressed {
                    self.buttons &= !0x04;
                } else {
                    self.buttons |= 0x04;
                }
            }
            Keycode::Return => {
                // bit 3 or the start button
                if pressed {
                    self.buttons &= !0x08;
                } else {
                    self.buttons |= 0x08;
                }
            }
            _ => {}
        }
    }

    // a dedicated function for the MMU to get the current value
    pub fn read(&self) -> u8 {
        let select_dpad = (self.reg & 0x10) == 0;
        let select_buttons = (self.reg & 0x20) == 0;

        let mut result = self.reg | 0xC0;

        if select_dpad && select_buttons {
            result = (result & 0xF0) | ((self.dpad & self.buttons) & 0x0F);
        } else if select_dpad {
            result = (result & 0xF0) | (self.dpad & 0x0F);
        } else if select_buttons {
            result = (result & 0xF0) | (self.buttons & 0x0F);
        } else {
            result |= 0x0F;
        }

        result
    }

    // dedicated function to write to the register from the MMU
    pub fn write(&mut self, value: u8) {
        self.reg = value | 0xC0;
    }
}
