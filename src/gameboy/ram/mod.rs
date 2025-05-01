pub struct RAM {
    rom: Vec<u8>, //Box<[u8; 0x8000]>,
    pub vram: [u8; 0x2000],
    pub wram: [u8; 0x2000],
    pub eram: [u8; 0x8000], // External Cartridge RAM
    pub rom_bank: usize,    // Active ROM Bank
    pub hram: [u8; 0x7F],
    pub oam: [u8; 0xA0],
    pub io: [u8; 0x80],
    pub interrupt_enable: u8,
}

impl RAM {
    pub fn new(rom: Vec<u8>) -> Self {
        //let mut rom = [0; 0x8000]; // Initialize with zeroed data
        //rom[..rom_data.len()].copy_from_slice(&rom_data); // Copy ROM contents
        Self {
            rom, //: Box::new(rom),
            vram: [0; 0x2000],
            wram: [0; 0x2000],
            eram: [0; 0x8000],
            rom_bank: 1,
            hram: [0; 0x7F],
            oam: [0; 0xA0],
            io: [0; 0x80],
            interrupt_enable: 0,
        }
    }
    pub fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom[address as usize],
            0x4000..=0x7FFF => {
                let banked_addr = (self.rom_bank * 0x4000) + (address as usize - 0x4000);
                self.rom.get(banked_addr).copied().unwrap_or(0xFF)
            }
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize],
            0xA000..=0xBFFF => self.eram[(address - 0xA000) as usize],
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize],
            0xE000..=0xFDFF => self.wram[(address - 0xE000) as usize],
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize],
            0xFF00..=0xFF7F => self.io[(address - 0xFF00) as usize],
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],
            0xFFFF => self.interrupt_enable,
            _ => 0x00,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        //bllargs test output
        if address == 0xFF02 && value == 0x81 {
            self.handle_serial_output();
        }

        match address {
            // ROM Bank Switching
            0x2000..=0x3FFF => self.rom_bank = (value & 0x1F) as usize,

            // External Cartridge RAM Writes
            0xA000..=0xBFFF => {
                if let Some(slot) = self.eram.get_mut((address - 0xA000) as usize) {
                    *slot = value;
                }
            }

            // Other memory writes remain unchanged
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
            0xFF04 => {
                if let Some(slot) = self.io.get_mut((0x4) as usize) {
                    *slot = 0x00;
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
            _ => {} // Ignore unmapped writes
        }
    }
    fn handle_serial_output(&self) {
        // Read the value from 0xFF01 (Serial Data Register)
        let data = self.io[0x01]; // Offset 0x01 in the I/O range corresponds to 0xFF01
        print!("{}", data as char); // Output as an ASCII character
    }

    pub fn update_div(&mut self, new: u8) {
        if let Some(slot) = self.io.get_mut((0x4) as usize) {
            *slot = new;
        }
    }
}
