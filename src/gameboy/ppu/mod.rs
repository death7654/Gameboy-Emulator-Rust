use std::cell::RefCell;
use std::rc::Rc;

use super::ram::RAM;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

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

pub struct PPU {
    /// 160×144 RGB framebuffer
    pub framebuffer: [u8; 160 * 144 * 3],
    pub ram: Rc<RefCell<RAM>>,
    pub scanline: u8,
    pub cycles: u32,
    lcd_on: bool,
    window_tile_map_area: u32,
    window_enable: bool,
    background_and_window_tile_area: u32,
    background_tilemap_area: u32,
    object_size: bool,
    object_enabled: bool,
    background_and_window_enable_priority: bool

}

impl PPU {
    pub fn new(ram: Rc<RefCell<RAM>>) -> Self {
        Self {
            framebuffer: [0; 160 * 144 * 3],
            ram,
            scanline: 0,
            cycles: 0,
            lcd_on: true,
            window_tile_map_area:0,
            window_enable: false,
            background_and_window_tile_area: 0,
            background_tilemap_area: 0,
            object_size: false,
            object_enabled: false,
            background_and_window_enable_priority: false
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
        //get data from lcd data register
        let lcd_control = self.ram.borrow().read(0xFF40);

        
        // bit 7: LCD power
        self.lcd_on = lcd_control & 0x80 != 0;
        self.framebuffer = [255; 160 * 144 * 3]; //sets the background as white regardless of value
        if !self.lcd_on {
            return;
        }

        // bit 6: Window tile map region.
        let window_tile_map_area = lcd_control & 0x40 != 0;
        self.window_tile_map_area = if window_tile_map_area {
            BG_MAP_SIGNED as u32
        } else {
            BG_MAP_UNSIGNED as u32
        };

        // bit 5: window enabled
        self.window_enable = lcd_control & 0x20 != 0;

        //bit 4: background tilemap area
        let background_and_window_tile_area = lcd_control & 0x10 != 0;
        self.background_and_window_tile_area = if background_and_window_tile_area {
            TILE_DATA_SIGNED as u32
        } else {
            TILE_DATA_UNSIGNED as u32
        };

        //bit 3: background tile map region
        let bg_tile_map_region = lcd_control & 0x08 != 0;
        self.background_tilemap_area = if bg_tile_map_region {
            BG_MAP_SIGNED as u32
        } else {
            BG_MAP_UNSIGNED as u32
        };

        //bit 2: sprite size
        self.object_size = lcd_control & 0x04 != 0;

        //bit 1: sprites enabled
        self.object_enabled = lcd_control & 0x02 != 0;

        //bit 0: background and window enable priority
        self.background_and_window_enable_priority = lcd_control & 0x01 != 0;
     
        // Rendering code goes here.
        self.render_background();
    }

    fn render_background(&mut self)
    {

        for y in 1..HEIGHT
        {
            let first_digit = self.ram.borrow().read((self.background_and_window_tile_area + (y-1)) as u16);
            let second_digit = self.ram.borrow().read((self.background_and_window_tile_area + y) as u16);

            for x in 0..WIDTH
            {
                  for pixel_x in 0..7
                {
                    let bit_index = 7-pixel_x;
                    let low_bit = (first_digit >> bit_index) & 1;
                    let high_bit = (second_digit >> bit_index) & 1;
                    let color_id = (high_bit<<1) | low_bit;

                    self.framebuffer[(x*y)] = DMG_BG_PALETTE[color_id as usize];

                }

            }

        }
    }
    fn render_tile(x: i32, y: i32, bg_x: i32, bg_y: i32) {}

    /// Expose framebuffer for display.
    pub fn get_framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }
}
