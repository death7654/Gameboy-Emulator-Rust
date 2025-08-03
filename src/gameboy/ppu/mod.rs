use std::cell::RefCell;
use std::rc::Rc;

use super::ram::RAM;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

const NUM_TILES: usize = 512;
const NUM_OBJECTS: u16 = 40;

type Tile = [u8; 64];

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
        let empty_tile = [255u8; 8 * 8];
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
        self.scanline_cycle += 4;

        // determine the current mode based on scanline_cycle position
        let previous_mode = self.mode;
        let new_mode = if self.scanline >= 144 {
            1 // VBlank
        } else if self.scanline_cycle < MODE2_CYCLES {
            2 // OAM search
        } else if self.scanline_cycle < MODE2_CYCLES + MODE3_CYCLES {
            3 // Pixel transfer
        } else {
            0 // HBlank
        };

        // handle mode changes
        if new_mode != previous_mode {
            self.mode = new_mode;
            self.on_mode_change();

            // render when entering Mode 3
            if new_mode == 3 && self.scanline < 144 && self.lcd_on {
                self.render_scanline(self.scanline as u16);
            }
        }

        // handle scanline completion (advance to next scanline)
        if self.scanline_cycle >= SCANLINE_CYCLES {
            // advance to next scanline
            self.scanline = self.scanline.wrapping_add(1);
            self.scanline_cycle -= SCANLINE_CYCLES;
            self.ram.borrow_mut().write(0xFF44, self.scanline);

            // VBlank interrupt at LY == 144
            if self.scanline == 144 && self.lcd_on {
                let mut ram = self.ram.borrow_mut();
                let curr_if = ram.read(0xFF0F);
                ram.write(0xFF0F, curr_if | 0x01);
            }
            else if self.scanline > 153 {
                self.scanline = 0;
            }

            let final_mode = if self.scanline >= 144 {
                1 
            } else {
                2 
            };

            if final_mode != self.mode {
                self.mode = final_mode;
                self.on_mode_change();
            }
        }
    }
    fn on_mode_change(&mut self) {
        let mut ram = self.ram.borrow_mut();
        // update STAT mode bits 0-1, preserving coincidence bit (bit 2)
        let mut stat = ram.read(0xFF41) & 0xF8;
        stat |= self.mode & 0x03;
        ram.write(0xFF41, stat);

        // STAT interrupts check bits 5,4,3 for modes 2,1,0 respectively and attempts to handle ram blocking
        let stat = ram.read(0xFF41);
        let request_stat = match self.mode {
            3 => {
                //ram.vram_blocked = true;
                //ram.oma_blocked = true;
                false
            }
            2 => {
                //ram.vram_blocked = false;
                //ram.oma_blocked = true;
                (stat & (1 << 5)) != 0
            }
            1 => {
                //ram.vram_blocked = false;
                //ram.oma_blocked = false;
                (stat & (1 << 4)) != 0
            }
            0 => {
                //ram.vram_blocked = false;
                //ram.oma_blocked = false;
                (stat & (1 << 3)) != 0
            }
            _ => false,
        };
        if request_stat {
            let curr_if = ram.read(0xFF0F);
            ram.write(0xFF0F, curr_if | 0x02);
        }
    }
    pub fn check_status(&mut self) {
        let lcd_control = self.ram.borrow().read(0xFF40);

        // bit 7: lcd on or off indicator
        self.lcd_on = lcd_control & 0x80 != 0;
        if !self.lcd_on {
            // return if lcd is off
            return;
        }

        // bit 6: where the tile maps are stored
        self.window_tile_map_area = if lcd_control & 0x40 != 0 {
            BG_MAP_SIGNED
        } else {
            BG_MAP_UNSIGNED
        };

        // bit 5: enable windows
        self.window_enable = lcd_control & 0x20 != 0;

        // bit 4: tile data area
        self.background_and_window_tile_area = if lcd_control & 0x10 != 0 {
            TILE_DATA_UNSIGNED
        } else {
            TILE_DATA_SIGNED
        };

        // bit 3: background tile area
        self.background_tilemap_area = if lcd_control & 0x08 != 0 {
            BG_MAP_SIGNED
        } else {
            BG_MAP_UNSIGNED
        };

        // bit 2: object size 8x8 or 8x16
        self.object_size = lcd_control & 0x04 != 0;

        // bit 1: objected enabled?
        self.object_enabled = lcd_control & 0x02 != 0;

        // bit 0: background and window enable priorty
        self.background_and_window_enable_priority = lcd_control & 0x01 != 0;

        self.render();
    }

    fn render(&mut self) {
        // regenerate tiles if the vram has been changed
        if self.ram.borrow().vram_changed {
        self.generate_tiles();
        }

        //render objects
        if self.object_enabled {
            self.render_objects();
        }
    }

    fn generate_tiles(&mut self) {
        // Generate tiles for both addressing modes
        // Tiles 0-255 for unsigned mode (0x8000-0x8FFF)
        // Tiles 0-127 for signed mode (0x9000-0x97FF)
        // Tiles 128-255 for signed mode (0x8800-0x8FFF)

        for tile_index in 0..256 {
            // calculate base address for this tile
            let base_unsigned = 0x8000 + (tile_index * 16) as u16;

            // generate tile for unsigned addressing mode
            self.generate_single_tile(base_unsigned, tile_index);

            // for signed addressing mode, we need to map tile indices differently
            if tile_index < 128 {
                // tiles 0-127 in signed mode come from 0x9000-0x97FF
                let base_signed = 0x9000 + (tile_index * 16) as u16;
                self.generate_single_tile(base_signed, 256 + tile_index);
            } else {
                // tiles 128-255 in signed mode come from 0x8800-0x8FFF
                let base_signed = 0x8800 + ((tile_index - 128) * 16) as u16;
                self.generate_single_tile(base_signed, 256 + tile_index);
            }
        }
    }

    fn generate_single_tile(&mut self, base_addr: u16, cache_index: usize) {
        if cache_index >= self.tile_cache.len() {
            return; 
        }

        // create a tile
        let mut tile = [0u8; 64];

        for y in 0..8 {
            // get the tile data from the ram
            let byte1 = self.ram.borrow_mut().read(base_addr + y * 2);
            let byte2 = self.ram.borrow_mut().read(base_addr + y * 2 + 1);

            for x in 0..8 {
                let bit = 7 - x;
                let lo = (byte1 >> bit) & 1;
                let hi = (byte2 >> bit) & 1;
                let color_index = (hi << 1) | lo;
                tile[y as usize * 8 + x as usize] = color_index;
            }
        }

        self.tile_cache[cache_index] = tile;
    }

    fn get_tile_for_rendering(&self, tile_index: u8, use_signed_addressing: bool) -> &Tile {
        if use_signed_addressing {
            // signed addressing, hence the offset
            &self.tile_cache[256 + tile_index as usize]
        } else {
            // unsigned addressing
            &self.tile_cache[tile_index as usize]
        }
    }

    // matches color index with data
    fn map_color_index(&self, index: u8, palette: u8) -> [u8; 3] {
        if index > 3 {
            return DMG_BG_PALLETE[0];
        }
        let shift = index * 2;
        let shade = (palette >> shift) & 0x03;
        DMG_BG_PALLETE[shade as usize]
    }

    fn render_scanline(&mut self, y: u16) {
        let ram = self.ram.borrow();

        let scy = ram.read(0xFF42) as u16;
        let scx = ram.read(0xFF43) as u16;

        let wy = ram.read(0xFF4A) as u16;
        let wx_raw = ram.read(0xFF4B);
        let wx = if wx_raw >= 7 { wx_raw - 7 } else { 0 } as u16;
        let using_window = self.window_enable && y >= wy && wy < 144;

        let bg_palette = ram.read(0xFF47);
        let use_signed_addressing = self.background_and_window_tile_area == TILE_DATA_SIGNED;

        for x in 0..WIDTH as u16 {
            let (tile_map_area, tile_x, tile_y, pixel_x, pixel_y) = if using_window && x >= wx {
                // window rendering
                let window_x = x - wx;
                let window_y = y - wy;

                let tile_col = window_x / 8;
                let tile_row = window_y / 8;
                let px = window_x % 8;
                let py = window_y % 8;

                (self.window_tile_map_area, tile_col, tile_row, px, py)
            } else {
                // background rendering
                let map_y = (scy + y) % 256;
                let map_x = (scx + x) % 256;

                let tile_col = map_x / 8;
                let tile_row = map_y / 8;
                let px = map_x % 8;
                let py = map_y % 8;

                (self.background_tilemap_area, tile_col, tile_row, px, py)
            };

            let tile_index_addr = tile_map_area + tile_y * 32 + tile_x;
            let tile_index = ram.read(tile_index_addr);

            // get the correct tile based on addressing mode
            let tile = self.get_tile_for_rendering(tile_index, use_signed_addressing);
            let color_index = tile[(pixel_y * 8 + pixel_x) as usize];
            let color = self.map_color_index(color_index, bg_palette);

            // draw pixel to framebuffer
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

        let obj_palette0 = self.ram.borrow().read(0xFF48);
        let obj_palette1 = self.ram.borrow().read(0xFF49);

        let ram = self.ram.borrow();

        for i in 0..NUM_OBJECTS {
            let offset = base + i * 4;
            let y_pos = ram.read(offset).wrapping_sub(16);
            let x_pos = ram.read(offset + 1).wrapping_sub(8);

            if y_pos >= 160 || x_pos >= 168 {
                continue;
            }

            let tile_index = ram.read(offset + 2) as usize;
            let attributes = ram.read(offset + 3);

            let y_flip = attributes & 0x40 != 0;
            let x_flip = attributes & 0x20 != 0;
            let palette = if attributes & 0x10 != 0 {
                obj_palette1
            } else {
                obj_palette0
            };
            let priority = attributes & 0x80 != 0;

            for tile_y in 0..sprite_height {
                let screen_y = y_pos as usize + tile_y;
                if screen_y >= HEIGHT as usize {
                    continue;
                }

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
                    let color_index = tile[tile_row * 8 + col];

                    // transparent pixel
                    if color_index == 0 {
                        continue;
                    }

                    if priority {
                        let i = (screen_y * 160 + screen_x) * 3;
                        let bg_pixel = &self.framebuffer[i..i + 3];
                        if bg_pixel != DMG_BG_PALLETE[0] {
                            continue;
                        }
                    }

                    let color = self.map_color_index(color_index, palette);
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
