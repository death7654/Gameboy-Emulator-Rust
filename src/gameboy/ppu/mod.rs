use std::cell::RefCell;
use std::rc::Rc;

use super::ram::RAM;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

const NUM_TILES: usize = 384;
const NUM_OBJECTS: u16 = 40;

type Tile = [[u8; 3]; 64];

//orginal gameboys background pallete
const DMG_BG_PALLETE: [[u8; 3]; 4] = [[255, 255, 255], [170, 170, 170], [85, 85, 85], [0, 0, 0]];

//background tile map
const BG_MAP_UNSIGNED: u16 = 0x9800;
const BG_MAP_SIGNED: u16 = 0x9C00;

//tile data
const TILE_DATA_SIGNED: u16 = 0x8800;
const TILE_DATA_UNSIGNED: u16 = 0x8000;

//ppu modes
const MODE2_CYCLES: u32 = 80;
const MODE3_CYCLES: u32 = 172;
const MODE0_CYCLES: u32 = 204;
const SCANLINE_CYCLES: u32 = MODE2_CYCLES + MODE3_CYCLES + MODE0_CYCLES;

pub struct PPU {
    pub framebuffer: [u8; 160 * 144 * 3],
    pub ram: Rc<RefCell<RAM>>,
    pub scanline: u8,
    pub scanline_cycle: u32,

    //stores each tile in a cache so it does not have to be recalculated
    tile_cache: Vec<Tile>,

    // lcd stats
    lcd_on: bool,
    window_tile_map_area: u16,
    window_enable: bool,
    background_and_window_tile_area: u16,
    background_tilemap_area: u16,
    object_size: bool,
    object_enabled: bool,
    background_and_window_enable_priority: bool,

    // current mode of the ppu
    mode: u8,
}

impl PPU {
    pub fn new(ram: Rc<RefCell<RAM>>) -> Self {
        let empty_tile = [[255u8; 3]; 8 * 8];
        let tile_cache = vec![empty_tile; NUM_TILES];

        ram.borrow_mut().write(0xFF44, 0);
        Self {
            framebuffer: [0; 160 * 144 * 3],
            ram,
            scanline: 0,
            scanline_cycle: 0,
            tile_cache,

            lcd_on: true,
            window_tile_map_area: 0,
            window_enable: false,
            background_and_window_tile_area: 0,
            background_tilemap_area: 0,
            object_size: false,
            object_enabled: false,
            background_and_window_enable_priority: false,

            mode: 2,
        }
    }

    pub fn step(&mut self) {
        // Add 4 T-cycles
        self.scanline_cycle += 4;
        if self.scanline_cycle >= SCANLINE_CYCLES {
            if self.scanline < 144 && self.lcd_on {
                // Render the current scanline before incrementing LY
                self.render_scanline(self.scanline as u16);
            }

            // Advance LY
            self.scanline = self.scanline.wrapping_add(1);
            self.scanline_cycle -= SCANLINE_CYCLES;

            // On entering VBlank at LY == 144, request VBlank interrupt
            if self.scanline == 144 && self.lcd_on {
                let mut ram = self.ram.borrow_mut();
                let curr_if = ram.read(0xFF0F);
                ram.write(0xFF0F, curr_if | 0x01);
            }
            // Wrap from 153→0
            else if self.scanline > 153 {
                self.scanline = 0;
            }

            // Write LY register
            self.ram.borrow_mut().write(0xFF44, self.scanline);

            // Determine new mode based on new scanline and remaining scanline_cycle
            let previous_mode = self.mode;
            let new_mode = if self.scanline >= 144 {
                // VBlank period
                1
            } else {
                // Visible scanlines 0..143
                if self.scanline_cycle < MODE2_CYCLES {
                    // Mode 2: OAM search
                    // Block OAM, allow VRAM
                    let mut ram = self.ram.borrow_mut();
                    ram.vram_blocked = false;
                    ram.oma_blocked = true;
                    2
                } else if self.scanline_cycle < MODE2_CYCLES + MODE3_CYCLES {
                    // Mode 3: Pixel transfer
                    // Block both VRAM and OAM
                    let mut ram = self.ram.borrow_mut();
                    ram.vram_blocked = true;
                    ram.oma_blocked = true;
                    3
                } else {
                    // Mode 0: HBlank
                    // Allow both VRAM and OAM
                    let mut ram = self.ram.borrow_mut();
                    ram.vram_blocked = false;
                    ram.oma_blocked = false;
                    0
                }
            };

            if new_mode != previous_mode {
                self.mode = new_mode;
                self.on_mode_change(); // handle STAT interrupts if needed
            }

           
            if self.mode == 3 && self.scanline < 144 && self.lcd_on {
                self.render_scanline(self.scanline as u16);
            }
        } else {
            let previous_mode = self.mode;
            let new_mode = if self.scanline >= 144 {
                1
            } else if self.scanline_cycle < MODE2_CYCLES {
                2
            } else if self.scanline_cycle < MODE2_CYCLES + MODE3_CYCLES {
                3
            } else {
                0
            };
            if new_mode != previous_mode {
                self.mode = new_mode;
                self.on_mode_change();

                // If we just entered Mode 3 mid-scanline, render that scanline now:
                if new_mode == 3 && self.scanline < 144 && self.lcd_on {
                    self.render_scanline(self.scanline as u16);
                }
            }
        }
    }
    fn on_mode_change(&mut self) {
        let mut ram = self.ram.borrow_mut();
        // Update STAT mode bits 0-1, preserving coincidence bit (bit 2)
        let mut stat = ram.read(0xFF41) & 0xF8;
        stat |= self.mode & 0x03;
        ram.write(0xFF41, stat);

        // STAT interrupts: check bits 5,4,3 for modes 2,1,0 respectively
        let stat = ram.read(0xFF41);
        let request_stat = match self.mode {
            2 => (stat & (1 << 5)) != 0,
            1 => (stat & (1 << 4)) != 0,
            0 => (stat & (1 << 3)) != 0,
            _ => false,
        };
        if request_stat {
            let curr_if = ram.read(0xFF0F);
            ram.write(0xFF0F, curr_if | 0x02);
        }
    }
    pub fn check_status(&mut self)
    {
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

        self.render();

    }

    fn render(&mut self) {

        //regenerate tiles if the vram has been changed
        if self.ram.borrow().vram_changed {
            self.generate_tiles();
        }

        //render objects
        if self.object_enabled {
            self.render_objects();
        }
    }

    fn generate_tiles(&mut self) {
        let ram = self.ram.borrow();
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
                let byte1 = ram.read(base + y * 2);
                let byte2 = ram.read(base + y * 2 + 1);

                //calculate colors
                for x in 0..8 {
                    let bit = 7 - x;
                    let lo = (byte1 >> bit) & 1;
                    let hi = (byte2 >> bit) & 1;
                    let color = (hi << 1) | lo;
                    tile[(y * 8) as usize + x] = DMG_BG_PALLETE[color as usize];
                }
            }

            self.tile_cache[tile_index] = tile;
        }
    }

    fn render_scanline(&mut self, y: u16) {
        let ram = self.ram.borrow();
        // figure out the offset
        let scy = ram.read(0xFF42) as u16;
        let scx = ram.read(0xFF43);

        // gameboy uses wrapping display viewports
        let map_y = (scy + y) % 256;
        let tile_row = map_y / 8;
        let pixel_y = map_y % 8;

        for x in 0..WIDTH as u16 {
            let map_x = (scx as u16 + x as u16) % 256;
            let tile_col = map_x / 8;
            let pixel_x = map_x % 8;

            let tile_index_addr = self.background_tilemap_area + tile_row * 32 + tile_col as u16;
            let mut tile_index = ram.read(tile_index_addr);

            if self.background_and_window_tile_area == TILE_DATA_SIGNED && tile_index < 128 {
                //if using the signed version add 256
                tile_index = tile_index.wrapping_add(255).wrapping_add(1);
            }

            let color =
                self.tile_cache[tile_index as usize][(pixel_y * 8) as usize + pixel_x as usize];
            let i = ((y * 160) as usize + x as usize) * 3;
            if i + 2 < self.framebuffer.len() {
                self.framebuffer[i] = color[0];
                self.framebuffer[i + 1] = color[1];
                self.framebuffer[i + 2] = color[2];
            }
        }
    }

    fn render_objects(&mut self) {
        let base = 0xFE00;
        let sprite_height = if self.object_size { 16 } else { 8 };

        let ram = self.ram.borrow();

        for i in 0..NUM_OBJECTS {
            //gets the next object
            let offset = base + i * 4;
            //reads the bytes
            //subtracts 16 from y position
            let y_pos = ram.read(offset).wrapping_sub(16);
            //subtracts 8 from x position
            let x_pos = ram.read(offset + 1).wrapping_sub(8);
            // Skip off-screen sprites
            if y_pos > 160 || x_pos >= 168 {
                continue;
            }

            //finds the index of the tile
            let tile_index = ram.read(offset + 2) as usize;

            //figure out attributes of the object
            let attributes = ram.read(offset + 3);

            let y_flip = attributes & 0x40 != 0;
            let x_flip = attributes & 0x20 != 0;

            //let palette = if attributes & 0x10 != 0 { 1 } else { 0 };

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
                    if color == DMG_BG_PALLETE[3] {
                        continue;
                    }

                    // Skip if priority is set AND BG pixel is not white
                    if priority {
                        let i = (screen_y * 160 + screen_x) * 3;
                        let bg_pixel = &self.framebuffer[i..i + 3];

                        if bg_pixel != DMG_BG_PALLETE[0] {
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
