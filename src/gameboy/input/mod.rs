use sdl2::{keyboard::Keycode, sys::key_t};

enum KEYS {
    SELECT = 0b11011111,
    SelectDPad = 0b11101111,
    StartOrDown = 0b11110111,
    SelectOrUp = 0b11111011,
    BOrLeft = 0b11111101,
    AOrRight = 0b11111110,
}

pub fn get_input(key: Keycode, pressed: bool) -> u8 {
    let mut data = 0b11111111; // Default all bits set (no keys pressed)

    if pressed {
        match key {
            Keycode::W => {
                data &= KEYS::SelectOrUp as u8;
            } // W -> D-Pad Up
            Keycode::A | Keycode::B => {
                data &= KEYS::BOrLeft as u8;
            } // A or B -> B Button
            Keycode::S | Keycode::Z => {
                data &= KEYS::StartOrDown as u8;
            } // S or Z -> Start/Down
            Keycode::D | Keycode::Space => {
                data &= KEYS::AOrRight as u8;
            } // D or Space -> A Button
            _ => eprintln!("Invalid key: {:?}", key), // Handle invalid keys
        }
    } else {
        match key {
            Keycode::W => {
                data &= !(KEYS::SelectOrUp as u8);
            } // Release D-Pad Up
            Keycode::A | Keycode::B => {
                data &= !(KEYS::BOrLeft as u8);
            } // Release B Button
            Keycode::S | Keycode::Z => {
                data &= !(KEYS::StartOrDown as u8);
            } // Release Start/Down
            Keycode::D | Keycode::Space => {
                data &= !(KEYS::AOrRight as u8);
            } // Release A Button
            _ => eprintln!("Invalid key: {:?}", key), // Handle invalid keys
        }
    }

    data
}
