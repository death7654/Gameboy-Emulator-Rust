use std::cell::RefCell;
use std::rc::Rc;

use super::ram::RAM;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

const NUM_TILES: usize = 384;
const NUM_OBJECTS: u16 = 40;

type Tile = [[u8; 3]; 64];

const DMG_BG_PALETTE: [[u8; 3]; 4] = [[255, 255, 255], [170, 170, 170], [85, 85, 85], [0, 0, 0]];

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

        //render objects
        if self.object_enabled {
            self.render_objects();
        }
    }

    fn generate_tiles(&mut self) {
        //generate tiles for the maximum number of tiles
        for tile_index in 0..NUM_TILES {
            //identify the base depending on if the background bit is using signed data or unsigned data
            let base = if self.background_and_window_tile_area == TILE_DATA_SIGNED {
                // treat as signed index from -128 to 127
                let signed_index = tile_index as i8 as i16;
                (0x9000u16 as i16 + signed_index * 16) as u16
            } else {
                0x8000 + (tile_index * 16) as u16
            };

            // get the current tile stored at the current index
            let mut tile: Tile = self.tile_cache[tile_index];

            for y in 0..8 {
                let byte1 = self.ram.borrow().read(base + y * 2);
                let byte2 = self.ram.borrow().read(base + y * 2 + 1);

                //calculate colors
                for x in 0..8 {
                    let bit = 7 - x;
                    let lo = (byte1 >> bit) & 1;
                    let hi = (byte2 >> bit) & 1;
                    let color = (hi << 1) | lo;
                    tile[(y * 8) as usize + x] = DMG_BG_PALETTE[color as usize];
                }
            }

            self.tile_cache[tile_index] = tile;
        }
    }

    fn render_tiles(&mut self) {
        // figure out the offset
        let scy = self.ram.borrow().read(0xFF42) as usize;
        let scx = self.ram.borrow().read(0xFF43) as usize;

        for y in 0..HEIGHT as usize {
            // gameboy uses wrapping display viewports
            let map_y = (scy + y) % 256;
            let tile_row = map_y / 8;
            let pixel_y = map_y % 8;

            for x in 0..WIDTH as usize {
                let map_x = (scx + x) % 256;
                let tile_col = map_x / 8;
                let pixel_x = map_x % 8;

                let tile_index_addr =
                    self.background_tilemap_area + (tile_row * 32 + tile_col) as u16;
                let mut tile_index = self.ram.borrow().read(tile_index_addr);

                if self.background_and_window_tile_area == TILE_DATA_SIGNED && tile_index < 128 {
                    //if using the signed version add 256
                    tile_index = tile_index.wrapping_add(255).wrapping_add(1);
                }

                let color = self.tile_cache[tile_index as usize][pixel_y * 8 + pixel_x];
                let i = (y * 160 + x) * 3;
                self.framebuffer[i] = color[0];
                self.framebuffer[i + 1] = color[1];
                self.framebuffer[i + 2] = color[2];
            }
        }
    }

    fn render_objects(&mut self) {
        let base = 0xFE00;
        let sprite_height = if self.object_size { 16 } else { 8 };

        for i in 0..NUM_OBJECTS {
            //gets the next object
            let offset = base + i * 4;
            //reads the bytes
            //subtracts 16 from y position
            let y_pos = self.ram.borrow().read(offset).wrapping_sub(16);
            //subtracts 8 from x position
            let x_pos = self.ram.borrow().read(offset + 1).wrapping_sub(8);
            // Skip off-screen sprites
            if y_pos > 160 || x_pos >= 168 {
                continue;
            }

            //finds the index of the tile
            let tile_index = self.ram.borrow().read(offset + 2) as usize;

            //figure out attributes of the object
            let attributes = self.ram.borrow().read(offset + 3);

            let y_flip = attributes & 0x40 != 0;
            let x_flip = attributes & 0x20 != 0;
            let palette = if attributes & 0x10 != 0 { 1 } else { 0 };

            //deterimes if the tile is behind bg or in front of bg
            let priority = attributes & 0x80 != 0;

            for tile_y in 0..sprite_height {
                //determines where to start
                let screen_y = y_pos as usize + tile_y;
                if screen_y >= HEIGHT as usize {
                    continue;
                }

                //determines if the pixel should be flipped on its y axis
                let row = if y_flip {
                    sprite_height - 1 - tile_y
                } else {
                    tile_y
                };

                let tile = if sprite_height == 16 {
                    let actual_tile = tile_index & 0xFE;
                    self.tile_cache[actual_tile + row / 8]
                } else {
                    self.tile_cache[tile_index]
                };

                let tile_row = row % 8;

                for tile_x in 0..8 {
                    let screen_x = x_pos as usize + tile_x;
                    if screen_x >= WIDTH as usize {
                        continue;
                    }

                    let col = if x_flip { 7 - tile_x } else { tile_x };

                    let color = tile[tile_row * 8 + col];

                    // Treat [0,0,0] as transparent for sprite color index 0
                    if color == DMG_BG_PALETTE[3] {
                        continue;
                    }

                    // Skip if priority is set AND BG pixel is not white
                    if priority {
                        let i = (screen_y * 160 + screen_x) * 3;
                        let bg_pixel = &self.framebuffer[i..i + 3];

                        if bg_pixel != DMG_BG_PALETTE[0] {
                            continue;
                        }
                    }

                    // Write sprite pixel to framebuffer
                    let i = (screen_y * 160 + screen_x) * 3;
                    self.framebuffer[i] = color[0];
                    self.framebuffer[i + 1] = color[1];
                    self.framebuffer[i + 2] = color[2];
                }
            }
        }
    }

    pub fn get_framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }
}
