pub struct GPU {
    pub framebuffer: [u8; 160 * 144 * 3],
}
impl GPU {
    pub fn new() -> Self {
        Self {
            framebuffer: [0x00; 160 * 144 * 3],
        }
    }
    pub fn render(&mut self, vram: &[u8; 0x2000]) {
        let tile_map_base = 0x1800; // using BG tile map at 0x9800
        let tile_data_base = 0x0000; // using tile data starting at 0x8000
    
        for ty in 0..18 {
            for tx in 0..20 {
                let tile_index = vram[tile_map_base + ty * 32 + tx];
    
                let tile_offset = tile_data_base + (tile_index as usize) * 16;
                for row in 0..8 {
                    let low_byte = vram[tile_offset + row * 2];
                    let high_byte = vram[tile_offset + row * 2 + 1];
    
                    for col in 0..8 {
                        let hi = (high_byte >> (7 - col)) & 1;
                        let lo = (low_byte >> (7 - col)) & 1;
                        let color_id = (hi << 1) | lo;
    
                        let color = match color_id {
                            0 => [255, 255, 255], // white
                            1 => [170, 170, 170], // light gray
                            2 => [85, 85, 85],    // dark gray
                            3 => [0, 0, 0],       // black
                            _ => [255, 0, 255],   // error magenta
                        };
    
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
