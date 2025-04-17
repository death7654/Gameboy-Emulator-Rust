pub struct RAM {
    rom : Vec<u8>,
    vram: [u8; 0x2000],
    wram: [u8; 0x2000],
    hram: [u8; 0x7F],
    oam: [u8; 0xA0], 
    io: [u8; 0x80], 
    interrupt_enable: u8,
}
impl RAM {
    pub fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            vram: [0; 0x2000],
            wram: [0; 0x2000],
            hram: [0; 0x7F],
            oam: [0; 0xA0],
            io: [0; 0x80],
            interrupt_enable: 0,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => self.rom[address as usize],               // ROM
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize],   // VRAM
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize],   // WRAM
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize],    // OAM
            0xFF00..=0xFF7F => self.io[(address - 0xFF00) as usize],     // I/O
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],   // HRAM
            0xFFFF => self.interrupt_enable,
            _ => 0xFF, // Unusable or not implemented
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize] = value,
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize] = value,
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize] = value,
            0xFF00..=0xFF7F => self.io[(address - 0xFF00) as usize] = value,
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize] = value,
            0xFFFF => self.interrupt_enable = value,
            _ => {}
        }
    }
}