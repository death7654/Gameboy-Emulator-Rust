pub trait Cartridge {
    fn read(&mut self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}
pub struct MBC0
// rom only
{
    rom: Vec<u8>,
}
impl MBC0 {
    pub fn new(rom: Vec<u8>) -> Self {
        Self { rom }
    }
}
impl Cartridge for MBC0 {
    fn read(&mut self, address: u16) -> u8 {
        self.rom.get(address as usize).copied().unwrap_or(0xFF)
    }
    fn write(&mut self, _address: u16, _value: u8) {
        // writes are ignored;
    }
}

pub struct MBC1 {
    rom: Vec<u8>,
    eram: Vec<u8>,
    ram_enabled: bool,
    rom_bank_low: u8,
    rom_bank_high: u8,
    ram_bank: u8,
    banking_mode: u8,
}

impl MBC1 {
    /// rom: full ROM bytes
    /// eram_size: size in bytes for external MMU (e.g., 0, 0x800, 0x2000, 0x8000)
    pub fn new(rom: Vec<u8>, eram_size: usize) -> Self {
        Self {
            rom,
            eram: vec![0; eram_size],
            ram_enabled: false,
            rom_bank_low: 1, // default to 1 (so switchable bank is bank 1)
            rom_bank_high: 0,
            ram_bank: 0,
            banking_mode: 0,
        }
    }

    pub fn active_rom_bank(&self) -> usize {
        let lower = (self.rom_bank_low & 0x1F) as usize;
        let upper = (self.rom_bank_high & 0x03) as usize;

        let mut bank = if self.banking_mode == 0 {
            (upper << 5) | lower
        } else {
            lower
        };

        if bank == 0 {
            bank = 1;
        }

        let max_banks = (self.rom.len() + 0x3FFF) / 0x4000; // ceil
        if bank >= max_banks {
            bank = max_banks.saturating_sub(1).max(1);
        }

        bank
    }

    pub fn read_rom(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom.get(address as usize).copied().unwrap_or(0xFF),
            0x4000..=0x7FFF => {
                let bank = self.active_rom_bank();
                let bank_base = bank * 0x4000;
                let off = bank_base + (address as usize - 0x4000);
                self.rom.get(off).copied().unwrap_or(0xFF)
            }
            _ => 0xFF,
        }
    }
}

impl Cartridge for MBC1 {
    fn read(&mut self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => self.read_rom(address),
            0xA000..=0xBFFF => {
                if !self.ram_enabled || self.eram.is_empty() {
                    0xFF
                } else {
                    let bank = if self.banking_mode == 1 {
                        (self.ram_bank & 0x03) as usize
                    } else {
                        0
                    };
                    self.ram_enabled = false;
                    let offset = bank * 0x2000 + (address as usize - 0xA000);
                    self.eram.get(offset).copied().unwrap_or(0xFF)
                }
            }
            _ => 0xFF,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => {
                self.ram_enabled = (value & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                let mut low = value & 0x1F;
                if low == 0 {
                    low = 1;
                } // bank 0 not allowed for switchable area
                self.rom_bank_low = low;
            }
            0x4000..=0x5FFF => {
                if self.banking_mode == 0 {
                    self.rom_bank_high = value & 0x03;
                } else {
                    self.ram_bank = value & 0x03;
                }
            }
            0x6000..=0x7FFF => {
                // Banking mode select: 0 => ROM mode, 1 => MMU mode
                self.banking_mode = value & 0x01;
            }
            0xA000..=0xBFFF => {
                if self.ram_enabled && !self.eram.is_empty() {
                    let bank = if self.banking_mode == 1 {
                        (self.ram_bank & 0x03) as usize
                    } else {
                        0
                    };
                    let offset = bank * 0x2000 + (address as usize - 0xA000);
                    if offset < self.eram.len() {
                        self.eram[offset] = value;
                    }
                }
            }
            _ => {}
        }
    }
}
