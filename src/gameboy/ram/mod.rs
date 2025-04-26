pub struct RAM {
    rom: Vec<u8>,
    pub vram: [u8; 0x2000],
    pub wram: [u8; 0x2000],
    pub hram: [u8; 0x7F],
    pub oam: [u8; 0xA0],
    pub io: [u8; 0x80],
    pub interrupt_enable: u8,
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
            0x0000..=0x7FFF => self.rom.get(address as usize).copied().unwrap_or(0xFF),
            0x8000..=0x9FFF => self
                .vram
                .get((address - 0x8000) as usize)
                .copied()
                .unwrap_or(0xFF),
            0xC000..=0xDFFF => self
                .wram
                .get((address - 0xC000) as usize)
                .copied()
                .unwrap_or(0xFF),
            0xFE00..=0xFE9F => self
                .oam
                .get((address - 0xFE00) as usize)
                .copied()
                .unwrap_or(0xFF),
            0xFF00..=0xFF7F => self
                .io
                .get((address - 0xFF00) as usize)
                .copied()
                .unwrap_or(0xFF),
            0xFF80..=0xFFFE => self
                .hram
                .get((address - 0xFF80) as usize)
                .copied()
                .unwrap_or(0xFF),
            0xFFFF => self.interrupt_enable,
            _ => 0xFF, // Unusable or unmapped memory
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x8000..=0x9FFF => {
                if let Some(slot) = self.vram.get_mut((address - 0x8000) as usize) {
                    *slot = value;
                }
            }
            0xC000..=0xDFFF => {
                if let Some(slot) = self.wram.get_mut((address - 0xC000) as usize) {
                    *slot = value;
                }
            }
            0xFE00..=0xFE9F => {
                if let Some(slot) = self.oam.get_mut((address - 0xFE00) as usize) {
                    *slot = value;
                }
            }
            0xFF00..=0xFF7F => {
                if let Some(slot) = self.io.get_mut((address - 0xFF00) as usize) {
                    *slot = value;
                }
            }
            0xFF80..=0xFFFE => {
                if let Some(slot) = self.hram.get_mut((address - 0xFF80) as usize) {
                    *slot = value;
                }
            }
            0xFFFF => self.interrupt_enable = value,
            _ => {} // Ignore writes to unmapped memory
        }
    }
}
