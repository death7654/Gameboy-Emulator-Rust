use std::cell::RefCell;
use std::rc::Rc;

use super::mmu::MMU;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

const NUM_TILES: usize = 512;
const NUM_OBJECTS: u16 = 40;

type Tile = [u8; 64];

//original gameboy background palette
const DMG_BG_PALETTE: [[u8; 3]; 4] = [[255, 255, 255], [170, 170, 170], [85, 85, 85], [0, 0, 0]];

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
    pub ram: Rc<RefCell<MMU>>,
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
    
    // window line counter for proper window rendering
    window_line_counter: u8,
}

impl PPU {
    pub fn new(ram: Rc<RefCell<MMU>>) -> Self {
        let empty_tile = [0u8; 8 * 8];
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
            window_line_counter: 0,
        }
    }

    pub fn step(&mut self) {
        // Update LCD control register state first
        self.check_status();
        
        // If LCD is off, don't process anything
        if !self.lcd_on {
            return;
        }

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
            if new_mode == 3 && self.scanline < 144 {
                self.render_scanline();
            }
        }

        // handle scanline completion
        if self.scanline_cycle >= SCANLINE_CYCLES {
            // advance to next scanline
            self.scanline = self.scanline.wrapping_add(1);
            self.scanline_cycle -= SCANLINE_CYCLES;
            self.ram.borrow_mut().write(0xFF44, self.scanline);

            // Reset window line counter at start of frame
            if self.scanline == 0 {
                self.window_line_counter = 0;
            }

            // sending a vblank interrupt
            if self.scanline == 144 {
                let mut ram = self.ram.borrow_mut();
                let curr_if = ram.read(0xFF0F);
                ram.write(0xFF0F, curr_if | 0x01);
            } else if self.scanline > 153 {
                self.scanline = 0;
                self.window_line_counter = 0;
            }

            // implements non-static mode switching
            let final_mode = if self.scanline >= 144 { 1 } else { 2 };

            if final_mode != self.mode {
                self.mode = final_mode;
                self.on_mode_change();
            }
        }
    }
    
    fn on_mode_change(&mut self) {
        let mut ram = self.ram.borrow_mut();

        // update stat mode bits 0-1
        let mut stat = ram.read(0xFF41) & !0x03;
        stat |= self.mode & 0x03;
        ram.write(0xFF41, stat);

        // stat interrupts check bits 5,4,3 for modes 2,1,0 respectively
        let stat = ram.read(0xFF41);
        let request_stat = match self.mode {
            2 => (stat & (1 << 5)) != 0, // OAM interrupt
            1 => (stat & (1 << 4)) != 0, // VBlank interrupt
            0 => (stat & (1 << 3)) != 0, // HBlank interrupt
            _ => false,
        };

        // calls a stat interrupt
        if request_stat {
            let curr_if = ram.read(0xFF0F);
            ram.write(0xFF0F, curr_if | 0x02);
        }
    }
    
    pub fn check_status(&mut self) {
        let lcd_control = self.ram.borrow_mut().read(0xFF40);

        // bit 7: lcd on or off indicator
        let new_lcd_on = lcd_control & 0x80 != 0;
        if !new_lcd_on && self.lcd_on {
            // LCD turned off - reset state
            self.scanline = 0;
            self.scanline_cycle = 0;
            self.mode = 0;
            self.window_line_counter = 0;
            self.ram.borrow_mut().write(0xFF44, 0);
        }
        self.lcd_on = new_lcd_on;

        if !self.lcd_on {
            return;
        }

        // bit 6: window tile map area
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

        // bit 1: objects enabled?
        self.object_enabled = lcd_control & 0x02 != 0;

        // bit 0: background and window enable priority
        self.background_and_window_enable_priority = lcd_control & 0x01 != 0;
    }

    fn generate_tiles(&mut self) {
        // Generate tiles for both addressing modes
        for tile_index in 0..256 {
            // Unsigned addressing mode (0x8000-0x8FFF)
            let base_unsigned = 0x8000 + (tile_index * 16) as u16;
            self.generate_single_tile(base_unsigned, tile_index);

            // Signed addressing mode
            if tile_index < 128 {
                // Tiles 0-127 in signed mode come from 0x9000-0x97FF
                let base_signed = 0x9000 + (tile_index * 16) as u16;
                self.generate_single_tile(base_signed, 256 + tile_index);
            } else {
                // Tiles 128-255 in signed mode come from 0x8800-0x8FFF
                let base_signed = 0x8800 + ((tile_index - 128) * 16) as u16;
                self.generate_single_tile(base_signed, 256 + tile_index);
            }
        }
        
        // Mark VRAM as no longer changed
        self.ram.borrow_mut().vram_changed = false;
    }

    fn generate_single_tile(&mut self, base_addr: u16, cache_index: usize) {
        let mut ram = self.ram.borrow_mut();
        if cache_index >= self.tile_cache.len() {
            return;
        }

        let mut tile = [0u8; 64];

        for y in 0..8 {
            let byte1 = ram.read(base_addr + y * 2);
            let byte2 = ram.read(base_addr + y * 2 + 1);

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
            // For signed addressing, treat tile_index as signed
            let signed_index = tile_index as i8;
            let cache_index = if signed_index >= 0 {
                256 + signed_index as usize
            } else {
                // For negative indices, they map to the second half of the signed tile cache
                let index = signed_index.wrapping_add(127).wrapping_add(1);
                256 + 128 + index as usize
            };
            &self.tile_cache[cache_index.min(self.tile_cache.len() - 1)]
        } else {
            &self.tile_cache[tile_index as usize]
        }
    }

    fn map_color_index(&self, index: u8, palette: u8) -> [u8; 3] {
        if index > 3 {
            return DMG_BG_PALETTE[0];
        }

        let shift = index * 2;
        let shade = (palette >> shift) & 0x03;
        DMG_BG_PALETTE[shade as usize]
    }

    fn render_scanline(&mut self) {
        // Generate tiles if VRAM changed
        if self.ram.borrow().vram_changed {
            self.generate_tiles();
        }

        let mut ram = self.ram.borrow_mut();
        let y = self.scanline as u16;

        let scy = ram.read(0xFF42) as u16;
        let scx = ram.read(0xFF43) as u16;
        let wy = ram.read(0xFF4A) as u16;
        let wx_raw = ram.read(0xFF4B);
        let wx = if wx_raw >= 7 { wx_raw - 7 } else { 0 } as u16;

        let bg_palette = ram.read(0xFF47);
        let use_signed_addressing = self.background_and_window_tile_area == TILE_DATA_SIGNED;

        // Check if window should be rendered on this scanline
        let window_active = self.window_enable && y >= wy && wy < 144;
        
        drop(ram);

        for x in 0..WIDTH as u16 {
            let (tile_map_area, tile_x, tile_y, pixel_x, pixel_y) = 
                if window_active && x >= wx {
                    // Window rendering
                    let window_x = x - wx;
                    let window_y = self.window_line_counter as u16;

                    (
                        self.window_tile_map_area,
                        window_x / 8,
                        window_y / 8,
                        window_x % 8,
                        window_y % 8,
                    )
                } else {
                    // Background rendering with proper wrapping
                    let map_y = (scy.wrapping_add(y)) & 0xFF;
                    let map_x = (scx.wrapping_add(x)) & 0xFF;

                    (
                        self.background_tilemap_area,
                        map_x / 8,
                        map_y / 8,
                        map_x % 8,
                        map_y % 8,
                    )
                };

            let mut ram = self.ram.borrow_mut();
            let tile_index_addr = tile_map_area + tile_y * 32 + tile_x;
            let tile_index = ram.read(tile_index_addr);
            drop(ram);

            let tile = self.get_tile_for_rendering(tile_index, use_signed_addressing);
            let color_index = tile[(pixel_y * 8 + pixel_x) as usize];
            let color = self.map_color_index(color_index, bg_palette);

            // Draw pixel to framebuffer
            let i = ((y * 160) as usize + x as usize) * 3;
            if i + 2 < self.framebuffer.len() {
                self.framebuffer[i] = color[0];
                self.framebuffer[i + 1] = color[1];
                self.framebuffer[i + 2] = color[2];
            }
        }

        // Increment window line counter if window was active
        if window_active {
            self.window_line_counter = self.window_line_counter.wrapping_add(1);
        }

        // Render sprites after background
        if self.object_enabled {
            self.render_objects();
        }
    }

    fn render_objects(&mut self) {
        let mut ram = self.ram.borrow_mut();
        let base = 0xFE00;
        let sprite_height = if self.object_size { 16 } else { 8 };
        let obj_palette0 = ram.read(0xFF48);
        let obj_palette1 = ram.read(0xFF49);
        let current_scanline = self.scanline;
        
        
        // Collect sprites for current scanline
        let mut sprites_on_line = Vec::new();
        
        for i in 0..NUM_OBJECTS {
            let offset = base + i * 4;
            let y_pos = ram.read(offset);
            
            // Skip sprite if Y position is invalid
            if y_pos == 0 || y_pos >= 160 {
                continue;
            }
            
            let sprite_top = y_pos.wrapping_sub(16);
            let sprite_bottom = sprite_top.wrapping_add(sprite_height as u8);
            
            if current_scanline >= sprite_top && current_scanline < sprite_bottom {
                sprites_on_line.push(i);
                
                // Game Boy only renders 10 sprites per scanline
                if sprites_on_line.len() >= 10 {
                    break;
                }
            }
        }

        for &sprite_index in sprites_on_line.iter().rev() {
            let offset = base + sprite_index * 4;
            let y_pos = ram.read(offset);
            let x_pos = ram.read(offset + 1);
            let tile_index = ram.read(offset + 2);
            let attributes = ram.read(offset + 3);

            let sprite_y = y_pos.wrapping_sub(16);
            let sprite_x = x_pos.wrapping_sub(8);

            // Skip sprites that are completely off screen
            if x_pos == 0 || x_pos >= 168 {
                continue;
            }

            let y_flip = attributes & 0x40 != 0;
            let x_flip = attributes & 0x20 != 0;
            let use_palette1 = attributes & 0x10 != 0;
            let priority = attributes & 0x80 != 0;

            let palette = if use_palette1 { obj_palette1 } else { obj_palette0 };

            // Calculate which row of the sprite we're rendering
            let sprite_row = current_scanline.wrapping_sub(sprite_y);
            let tile_row = if y_flip {
                (sprite_height as u8 - 1).wrapping_sub(sprite_row)
            } else {
                sprite_row
            };

            // Get the correct tile - sprites always use unsigned addressing
            let actual_tile_index = if sprite_height == 16 {
                // For 8x16 sprites, ignore bit 0 of tile index and use consecutive tiles
                let base_tile = (tile_index & 0xFE) as usize;
                base_tile + (tile_row / 8) as usize
            } else {
                tile_index as usize
            };

            // Sprites always use the unsigned tile cache
            if actual_tile_index >= 256 {
                continue;
            }

            let tile = &self.tile_cache[actual_tile_index];
            let tile_y = tile_row % 8;

            // Render the sprite pixels
            for tile_x in 0..8 {
                let screen_x = sprite_x.wrapping_add(tile_x);
                
                // Skip if pixel is off screen
                if screen_x >= 160 {
                    continue;
                }

                let col = if x_flip { 7 - tile_x } else { tile_x };
                let color_index = tile[(tile_y * 8 + col) as usize];

                // Skip transparent pixels (color 0)
                if color_index == 0 {
                    continue;
                }

                let fb_index = (current_scanline as usize * 160 + screen_x as usize) * 3;
                if fb_index + 2 >= self.framebuffer.len() {
                    continue;
                }

                // Check priority flag
                if priority {
                    // Only draw over background color 0 
                    let bg_is_color0 = self.framebuffer[fb_index] == DMG_BG_PALETTE[0][0]
                        && self.framebuffer[fb_index + 1] == DMG_BG_PALETTE[0][1]
                        && self.framebuffer[fb_index + 2] == DMG_BG_PALETTE[0][2];
                    if !bg_is_color0 {
                        continue;
                    }
                }

                // Map sprite color using the sprite's palette
                let color = self.map_color_index(color_index, palette);
                self.framebuffer[fb_index] = color[0];
                self.framebuffer[fb_index + 1] = color[1];
                self.framebuffer[fb_index + 2] = color[2];
            }
        }
    }

    pub fn get_framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }
}