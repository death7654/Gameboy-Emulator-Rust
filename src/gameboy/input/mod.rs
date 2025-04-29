use sdl2::keyboard::Keycode;

const MASK_UP: u8 = 0b11111011;
const MASK_B_LEFT: u8 = 0b11111101;
const MASK_START_DOWN: u8 = 0b11110111;
const MASK_A_RIGHT: u8 = 0b11111110;
const MASK_SELECT_BUTTON: u8 = 0b11011111;
const MASK_SELECT_DPAD: u8 = 0b11101111;

pub fn get_input(key: Keycode, pressed: bool, current: u8) -> u8 {
    let mut data = current;
    match (key, pressed) {
        (Keycode::W, true) => data &= MASK_UP,
        (Keycode::W, false) => data |= 0b00000100,
        (Keycode::A, true) | (Keycode::B, true) => data &= MASK_B_LEFT,
        (Keycode::A, false) | (Keycode::B, false) => data |= 0b00000010,
        (Keycode::S, true) | (Keycode::Z, true) => data &= MASK_START_DOWN,
        (Keycode::S, false) | (Keycode::Z, false) => data |= 0b00001000,
        (Keycode::D, true) | (Keycode::Space, true) => data &= MASK_A_RIGHT,
        (Keycode::D, false) | (Keycode::Space, false) => data |= 0b00000001,
        (Keycode::Return, true) => data &= MASK_SELECT_BUTTON,
        (Keycode::Return, false) => data |= 0b00100000,
        (Keycode::Backslash, true) => data &= MASK_SELECT_DPAD,
        (Keycode::Backslash, false) => data |= 0b00010000,
        (_, _) => {}
    }
    data
}
