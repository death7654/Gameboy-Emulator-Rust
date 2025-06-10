use std::cell::RefCell;
use std::rc::Rc;

use super::ram::RAM;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

const NUM_TILES: usize = 384;
type Tile = [[u8; 3]; 64];

const DMG_BG_PALETTE: [[u8; 3]; 4] = [
    [255, 255, 255],
    [170, 170, 170],
    [85, 85, 85],
    [0, 0, 0],
];

const DMG_SPRITE_PALETTE: [[u8; 3]; 4] = [
    [0, 0, 0],       // Transparent (ignored)
    [255, 255, 255], // White
    [170, 170, 170], // Light gray
    [85, 85, 85],    // Dark gray
];

//background tile map
const BG_MAP_UNSIGNED: u16 = 0x9800;
const BG_MAP_SIGNED: u16 = 0x9C00;

//tile data
const TILE_DATA_SIGNED: u16 = 0x8800;
const TILE_DATA_UNSIGNED: u16 = 0x8000;

pub struct PPU {
    pub framebuffer: [u8; 160 * 144 * 3],
    pub ram: Rc<RefCell<RAM>>,
    pub scanline: u8,
    pub cycles: u32,
    tile_cache: [Tile; NUM_TILES],

    lcd_on: bool,
    window_tile_map_area: u16,
    window_enable: bool,
    background_and_window_tile_area: u16,
    background_tilemap_area: u16,
    object_size: bool,
    object_enabled: bool,
    background_and_window_enable_priority: bool,
}

impl PPU {
    pub fn new(ram: Rc<RefCell<RAM>>) -> Self {
        let tile: Tile = [[255; 3]; 64];
        Self {
            framebuffer: [0; 160 * 144 * 3],
            ram,
            scanline: 0,
            cycles: 0,
            tile_cache: [tile; NUM_TILES],

            lcd_on: true,
            window_tile_map_area: 0,
            window_enable: false,
            background_and_window_tile_area: 0,
            background_tilemap_area: 0,
            object_size: false,
            object_enabled: false,
            background_and_window_enable_priority: false,
        }
    }

    pub fn step(&mut self) {
        self.cycles += 4;
        if self.cycles > 456 {
            self.scanline = self.scanline.wrapping_add(1);
            self.cycles -= 456;

            if self.scanline == 144 && self.lcd_on {
                self.ram.borrow_mut().write(0xFF0F, 0b0000_0001);
            } else if self.scanline >= 154 {
                self.scanline = 0;
            }

            self.ram.borrow_mut().write(0xFF44, self.scanline);
        }
    }

    pub fn render(&mut self) {
        let lcd_control = self.ram.borrow().read(0xFF40);

        //bit 7: lcd on or off indicator
        self.lcd_on = lcd_control & 0x80 != 0;
        if !self.lcd_on {
            return;
        }

        //bit 6: where the tile maps are stored
        self.window_tile_map_area = if lcd_control & 0x40 != 0 {
            BG_MAP_SIGNED
        } else {
            BG_MAP_UNSIGNED
        };

        //bit 5: enable windows
        self.window_enable = lcd_control & 0x20 != 0;

        //bit 4: tile data area
        self.background_and_window_tile_area = if lcd_control & 0x10 != 0 {
            TILE_DATA_UNSIGNED
        } else {
            TILE_DATA_SIGNED
        };

        //bit 3: background tile area
        self.background_tilemap_area = if lcd_control & 0x08 != 0 {
            BG_MAP_SIGNED
        } else {
            BG_MAP_UNSIGNED
        };

        //bit 2: object size 8x8 or 8x16
        self.object_size = lcd_control & 0x04 != 0;

        //bit 1: objected enabled?
        self.object_enabled = lcd_control & 0x02 != 0;

        //bit 0: background and window enable priorty
        self.background_and_window_enable_priority = lcd_control & 0x01 != 0;

        //regenerate tiles if the vram has been changed
        if self.ram.borrow().vram_changed {
            self.generate_tiles();
        }

        //render tiles
        self.render_tiles();
    }

    fn generate_tiles(&mut self) {
        // for the tile index
        for tile_index in 0..NUM_TILES {
            //grab the current tile
            let mut tile: Tile = self.tile_cache[tile_index];

            let base = self.background_and_window_tile_area + (tile_index * 16) as u16;
            for y in 0..8 {
                //grab 2 bytes
                let byte1 = self.ram.borrow().read(base + y * 2);
                let byte2 = self.ram.borrow().read(base + y * 2 + 1);

                for x in 0..8 {
                    let bit = 7 - x;
                    let lo = (byte1 >> bit) & 1;
                    let hi = (byte2 >> bit) & 1;
                    let color = (hi << 1) | lo;
                    //calculate colors
                    tile[(y * 8) as usize + x] = DMG_BG_PALETTE[color as usize];
                }
            }
            self.tile_cache[tile_index] = tile;
        }
    }

    fn render_tiles(&mut self) {
        let scy = self.ram.borrow().read(0xFF42) as usize;
        let scx = self.ram.borrow().read(0xFF43) as usize;

        for y in 0..HEIGHT as usize {
            let map_y = (scy + y) % 256;
            let tile_row = map_y / 8;
            let pixel_y = map_y % 8;

            for x in 0..WIDTH as usize {
                let map_x = (scx + x) % 256;
                let tile_col = map_x / 8;
                let pixel_x = map_x % 8;

                let tile_index_addr = self.background_tilemap_area + (tile_row * 32 + tile_col) as u16;
                let mut tile_index = self.ram.borrow().read(tile_index_addr);

                if self.background_and_window_tile_area == TILE_DATA_SIGNED && tile_index < 128 {
                    tile_index = tile_index.wrapping_add(255);
                }

                let color = self.tile_cache[tile_index as usize][pixel_y * 8 + pixel_x];
                let i = (y * 160 + x) * 3;
                self.framebuffer[i] = color[0];
                self.framebuffer[i + 1] = color[1];
                self.framebuffer[i + 2] = color[2];
            }
        }
    }

    pub fn get_framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }
}
