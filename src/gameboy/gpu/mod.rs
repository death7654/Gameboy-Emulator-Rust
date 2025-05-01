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
const BG_MAP_UNSIGNED: usize = 0x9800; // if LCDC bit 3 = 0
const BG_MAP_SIGNED: usize = 0x9C00; // if LCDC bit 3 = 1
const TILE_DATA_UNSIGNED: usize = 0x8000; // if LCDC bit 4 = 1
const TILE_DATA_SIGNED: usize = 0x8800; // if LCDC bit 4 = 0

pub struct GPU {
    /// 160×144 RGB framebuffer
    pub framebuffer: [u8; 160 * 144 * 3],
    pub ram: Rc<RefCell<RAM>>,
}

impl GPU {
    pub fn new(ram: Rc<RefCell<RAM>>) -> Self {
        Self {
            framebuffer: [0; 160 * 144 * 3],
            ram,
        }
    }

    pub fn render(&mut self) {
        // —— Background Pass ——
        let ram = self.ram.borrow();
        let lcdc = ram.read(0xFF40);

        // LCD off: white screen
        if lcdc & 0x80 == 0 {
            self.framebuffer.fill(255);
            return;
        }

        // Scroll registers
        let scy = ram.read(0xFF42) as usize;
        let scx = ram.read(0xFF43) as usize;

        // Tile‐map select
        let bg_map_base = if lcdc & 0x08 != 0 {
            BG_MAP_SIGNED
        } else {
            BG_MAP_UNSIGNED
        };
        let bg_map_offset = bg_map_base - VRAM_START;

        // Tile‐data select + signed/unsigned mode
        let (tile_data_base, use_unsigned) = if lcdc & 0x10 != 0 {
            (TILE_DATA_UNSIGNED, true)
        } else {
            (TILE_DATA_SIGNED, false)
        };

        let vram = &ram.vram;
        for y in 0..144 {
            for x in 0..160 {
                // scrolled coordinates
                let bx = (x + scx) % 256;
                let by = (y + scy) % 256;
                let col = bx / 8;
                let row = by / 8;
                let map_idx = row * 32 + col;
                let tile_no = vram[bg_map_offset + map_idx];

                let tile_idx = if use_unsigned {
                    tile_no as usize
                } else {
                    (tile_no as i8 as isize + 128) as usize
                };
                let tile_off = (tile_data_base - VRAM_START) + tile_idx * 16;

                let px = bx % 8;
                let py = by % 8;
                let row_off = py * 2;
                if tile_off + row_off + 1 >= vram.len() {
                    continue;
                }

                let lo = (vram[tile_off + row_off] >> (7 - px)) & 1;
                let hi = (vram[tile_off + row_off + 1] >> (7 - px)) & 1;
                let color = DMG_BG_PALETTE[((hi << 1) | lo) as usize];

                let fb = (y * 160 + x) * 3;
                self.framebuffer[fb] = color[0];
                self.framebuffer[fb + 1] = color[1];
                self.framebuffer[fb + 2] = color[2];
            }
        }
        drop(ram); // free the immutable borrow

        // —— Window Pass ——
        self.render_window();

        // —— Sprite Pass ——
        let ram = self.ram.borrow();
        let lcdc = ram.read(0xFF40);
        if lcdc & 0x02 != 0 {
            let oam = &ram.oam;
            let sprite_h = if lcdc & 0x04 != 0 { 16 } else { 8 };

            for i in 0..40 {
                let base = i * 4;
                let sy = oam[base] as i16 - 16;
                let sx = oam[base + 1] as i16 - 8;
                let tile_no = oam[base + 2];
                let attrs = oam[base + 3];
                let x_flip = attrs & 0x20 != 0;
                let y_flip = attrs & 0x40 != 0;

                let vram = &ram.vram; // unsigned tile data
                let tile_off = (tile_no as usize) * 16;

                for row in 0..sprite_h {
                    let ty = if y_flip { sprite_h - 1 - row } else { row };
                    let off = tile_off + ty as usize * 2;
                    if off + 1 >= vram.len() {
                        continue;
                    }

                    let lo = vram[off];
                    let hi = vram[off + 1];
                    for col in 0..8 {
                        let tx = if x_flip { 7 - col } else { col };
                        let bit = 7 - tx;
                        let c = (((hi >> bit) & 1) << 1) | ((lo >> bit) & 1);
                        if c == 0 {
                            continue;
                        } // transparent

                        let px = sx + col as i16;
                        let py = sy + row as i16;
                        if px < 0 || px >= 160 || py < 0 || py >= 144 {
                            continue;
                        }

                        let color = DMG_SPRITE_PALETTE[c as usize];
                        let fb = (py as usize * 160 + px as usize) * 3;
                        self.framebuffer[fb] = color[0];
                        self.framebuffer[fb + 1] = color[1];
                        self.framebuffer[fb + 2] = color[2];
                    }
                }
            }
        }
    }

    /// Correctly handles WX<7 as a negative X origin,
    /// clips to [0..160)×[0..144), and never conflicts borrows.
    fn render_window(&mut self) {
        // pull everything out of RAM (including a VRAM clone) in one borrow
        let (use_unsigned, tile_data_base, map_off, pos_x, pos_y, vram) = {
            let ram = self.ram.borrow();
            let lcdc = ram.read(0xFF40);
            if lcdc & 0x20 == 0 {
                return;
            } // window disabled
            let raw_wy = ram.read(0xFF4A) as isize;
            let raw_wx = ram.read(0xFF4B) as isize;
            if raw_wy >= 144 {
                return;
            } // fully off-screen below

            // on-screen origin
            let pos_y = raw_wy;
            let pos_x = raw_wx - 7;

            let base = if lcdc & 0x40 != 0 {
                BG_MAP_SIGNED
            } else {
                BG_MAP_UNSIGNED
            };
            let map_off = base - VRAM_START;

            let (tile_data_base, use_unsigned) = if lcdc & 0x10 != 0 {
                (TILE_DATA_UNSIGNED, true)
            } else {
                (TILE_DATA_SIGNED, false)
            };

            // clone VRAM so we drop the borrow here
            let vram = ram.vram.clone();
            (use_unsigned, tile_data_base, map_off, pos_x, pos_y, vram)
        };

        let y0 = pos_y.max(0) as usize;
        let x0 = pos_x.max(0) as usize;
        let y1 = 144;
        let x1 = 160;

        for wy in y0..y1 {
            for wx in x0..x1 {
                let iy = (wy as isize - pos_y) as usize;
                let ix = (wx as isize - pos_x) as usize;

                let col = ix / 8;
                let row = iy / 8;
                let idx = row * 32 + col;
                if map_off + idx >= vram.len() {
                    continue;
                }

                let tn = vram[map_off + idx];
                let ti = if use_unsigned {
                    tn as usize
                } else {
                    (tn as i8 as isize + 128) as usize
                };
                let to = (tile_data_base - VRAM_START) + ti * 16;

                let py = iy % 8;
                let px = ix % 8;
                let ro = py * 2;
                if to + ro + 1 >= vram.len() {
                    continue;
                }

                let lo = (vram[to + ro] >> (7 - px)) & 1;
                let hi = (vram[to + ro + 1] >> (7 - px)) & 1;
                let color = DMG_BG_PALETTE[((hi << 1) | lo) as usize];

                let fb = (wy * 160 + wx) * 3;
                self.framebuffer[fb] = color[0];
                self.framebuffer[fb + 1] = color[1];
                self.framebuffer[fb + 2] = color[2];
            }
        }
    }

    /// Expose framebuffer for display
    pub fn get_framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }
}
