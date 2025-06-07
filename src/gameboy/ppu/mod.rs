use std::cell::RefCell;
use std::rc::Rc;

use super::ram::RAM;

// DMG palettes for background and sprites.
const DMG_BG_PALETTE: [[u8; 3]; 4] = [
    [255, 255, 255], // White
    [170, 170, 170], // Light gray
    [85, 85, 85],    // Dark gray
    [0, 0, 0],       // Black
];

// For sprites, color zero is transparent.
const DMG_SPRITE_PALETTE: [[u8; 3]; 4] = [
    [0, 0, 0],       // Transparent (ignored)
    [255, 255, 255], // White
    [170, 170, 170], // Light gray
    [85, 85, 85],    // Dark gray
];

// VRAM layout constants (slice index 0 == address 0x8000)
const VRAM_START: usize = 0x8000;

const BG_MAP_UNSIGNED: usize = 0x9800;
const BG_MAP_SIGNED: usize = 0x9C00;

const TILE_DATA_UNSIGNED: usize = 0x8800;
const TILE_DATA_SIGNED: usize = 0x8000;

pub struct GPU {
    /// 160×144 RGB framebuffer
    pub framebuffer: [u8; 160 * 144 * 3],
    pub ram: Rc<RefCell<RAM>>,
    pub scanline: u8,
    pub cycles: u32,
    lcd_on: bool,
}

impl GPU {
    pub fn new(ram: Rc<RefCell<RAM>>) -> Self {
        Self {
            framebuffer: [0; 160 * 144 * 3],
            ram,
            scanline: 0,
            cycles: 0,
            lcd_on: true,
        }
    }
    pub fn step(&mut self) {
        self.cycles += 4;
        if self.cycles > 456 {
            self.scanline = self.scanline.wrapping_add(1);
            self.cycles -= 456;

            let scanlines = self.scanline;

            //only allow interrupt if lcd is on
            if scanlines == 144 && self.lcd_on {
                self.ram.borrow_mut().write(0xFF0F, 0b0000_0001);
            }
            //reset scanlines
            else if scanlines >= 154 {
                self.scanline = 0;
            }
            //writing to ram
            self.ram.borrow_mut().write(0xFF44, self.scanline);
        }
    }

    pub fn render(&mut self) {
        let lcd_control = self.ram.borrow().read(0xFF40);

        let lcd_power = lcd_control & 0x80 != 0;
        let window_tile_map_region = lcd_control & 0x40 != 0;
        let window_enabled = lcd_control & 0x20 != 0;
        let bg_and_window_tileset_region = lcd_control & 0x10 != 0;
        let bg_tile_map_region = lcd_control & 0x08 != 0;
        let sprite_size = lcd_control & 0x04 != 0;
        let spries_enabled = lcd_control & 0x02 != 0;
        let bg_enabled = lcd_control & 0x01 != 0;

        // todo: sync with v_blank

        // bit 7: LCD power
        if !lcd_power {
            self.lcd_off();
            return;
        }

        // bit 6: Window tile map region.
        let window_tile_map = if window_tile_map_region {
            BG_MAP_SIGNED
        } else {
            BG_MAP_UNSIGNED
        };

        // bit 4: BG and window tileset region.
        let bg_window_tileset = if bg_and_window_tileset_region {
            TILE_DATA_SIGNED
        } else {
            TILE_DATA_UNSIGNED
        };

        // bit 3: Background tile map region.
        let bg_tile_map = if bg_tile_map_region {
            BG_MAP_SIGNED
        } else {
            BG_MAP_UNSIGNED
        };

        // todo: bit 2; sprite size;

        // Rendering code goes here.
    }
    fn lcd_off(&mut self) {
        self.framebuffer = [255; 160 * 144 * 3];
    }
    fn render_tile(x: i32, y: i32, bg_x: i32, bg_y: i32) {}

    /// Expose framebuffer for display.
    pub fn get_framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }
}
