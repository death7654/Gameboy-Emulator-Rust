use std::cell::RefCell;
use std::rc::Rc;

use super::ram::RAM;

const PALLETE: [[u8; 3]; 4] = [
    [255, 255, 255], // White
    [170, 170, 170], // Light Gray
    [85, 85, 85],    // Dark Gray
    [0, 0, 0],       // Black
];

pub struct GPU {
    pub framebuffer: [u8; 160 * 144 * 3],
    pub ram: Rc<RefCell<RAM>>,
}

impl GPU {
    pub fn new(ram: Rc<RefCell<RAM>>) -> Self {
        Self {
            framebuffer: [0xff; 160 * 144 * 3],
            ram,
        }
    }

    pub fn render(&mut self) {
        let ram = self.ram.borrow();
        let vram = &ram.vram;
        let lcdc = ram.read(0xFF40);

        let tile_map_base = if lcdc & 0x08 != 0 { 0x1C00 } else { 0x1800 };
        let tile_data_base = if lcdc & 0x10 != 0 { 0x0000 } else { 0x1000 };

        for ty in 0..18 {
            for tx in 0..20 {
                let tile_index = vram[tile_map_base + ty * 32 + tx];

                // Handle signed index for 0x8800 addressing mode
                let tile_offset = if tile_data_base == 0x1000 {
                    tile_data_base + ((tile_index as i8 as i16 + 128) as usize) * 16
                } else {
                    tile_data_base + (tile_index as usize) * 16
                };

                if tile_offset + 16 > vram.len() {
                    continue;
                }

                for row in 0..8 {
                    let low_byte = vram[tile_offset + row * 2];
                    let high_byte = vram[tile_offset + row * 2 + 1];

                    for col in 0..8 {
                        let bit_index = 7 - col;
                        let lo = (low_byte >> bit_index) & 1;
                        let hi = (high_byte >> bit_index) & 1;
                        let color_id = (hi << 1) | lo;
                        let color = PALLETE[color_id as usize];

                        let px = tx * 8 + col;
                        let py = ty * 8 + row;

                        if px < 160 && py < 144 {
                            let i = (py * 160 + px) * 3;
                            self.framebuffer[i] = color[0];
                            self.framebuffer[i + 1] = color[1];
                            self.framebuffer[i + 2] = color[2];
                        }
                    }
                }
            }
        }
    }

    pub fn get_framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }
}
