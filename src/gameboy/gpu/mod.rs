const PALLETE: [[u8; 3]; 4] = [
    [255, 255, 255], // White
    [170, 170, 170], // Light Gray
    [85, 85, 85],    // Dark Gray
    [0, 0, 0],       // Black
];

pub struct GPU {
    pub framebuffer: [u8; 160 * 144 * 3],
}
impl GPU {
    pub fn new() -> Self {
        Self {
            framebuffer: [0xff; 160 * 144 * 3],
        }
    }
    pub fn render(&mut self, mut vram: &[u8; 0x2000]) {
        let tile_map_base = 0x1800;
        let tile_data_base = 0x0000;

        println!("Tile Data[0x0000..0x0010]: {:?}", &vram[0x0000..0x0010]); // Tile 0
        println!("Tile Data[0x0010..0x0020]: {:?}", &vram[0x0010..0x0020]); // Tile 1
        println!("Tile Data[0x0020..0x0030]: {:?}", &vram[0x0020..0x0030]); // Tile 2


        for ty in 0..18 {
            for tx in 0..20 {
                let tile_index = vram[tile_map_base + ty * 32 + tx];
                let tile_offset = tile_data_base + (tile_index as usize) * 16;

                if tile_offset + 16 > vram.len() {
                    eprintln!("Invalid tile index: {}", tile_index);
                    continue;
                }

                for row in 0..8 {
                    let low_byte = vram[tile_offset + row * 2];
                    let high_byte = vram[tile_offset + row * 2 + 1];

                    for col in 0..8 {
                        let hi = (high_byte >> (7 - col)) & 1;
                        let lo = (low_byte >> (7 - col)) & 1;
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
