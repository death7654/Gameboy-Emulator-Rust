// basic implementation for cartridge
pub trait Cartridge {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

// rom only or Memory Bank Controller
pub struct MBC0 {
    rom: Vec<u8>,
}
impl MBC0 {
    pub fn new(rom: Vec<u8>) -> Self {
        Self { rom }
    }
}

// read/write implemenation for MBC0
impl Cartridge for MBC0 {
    fn read(&self, address: u16) -> u8 {
        self.rom.get(address as usize).copied().unwrap_or(0xFF)
    }
    fn write(&mut self, _address: u16, _value: u8) {
        // writes are ignored;
    }
}

/*
    MBC1
        0x0000 -> 0x3FFF => Rom Bank 0
        0x4000 -> 0x7FFF => Rom Bank 1
            - contains other Rom banks the ROM may use
        0xA000 -> 0xBFFF => RAM Bank 0 to 3
            - for extra ram



*/
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
    fn read(&self, address: u16) -> u8 {
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
                }
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
                // Banking mode select where 0 is ROM mode and 1 is RAM/MMU mode
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

/*
   MBC3
       - Very Similar to MBC1 but with added RTC/Real Time Clock
*/
pub struct MBC3 {
    rom: Vec<u8>,
    eram: Vec<u8>,
    ram_enabled: bool,
    rom_bank: u8,
    ram_bank: u8,
    rtc_selected: bool,
    rtc_reg: u8,
    rtc: [u8; 5],
    rtc_latched: [u8; 5],
    latch_flag: bool,
}

impl MBC3 {
    pub fn new(rom: Vec<u8>, eram_size: usize) -> Self {
        Self {
            rom,
            eram: vec![0; eram_size],
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0,
            rtc_selected: false,
            rtc_reg: 0,
            rtc: [0; 5],
            rtc_latched: [0; 5],
            latch_flag: false,
        }
    }

    fn active_rom_bank(&self) -> usize {
        let mut bank = self.rom_bank as usize;
        if bank == 0 {
            bank = 1;
        }
        let max_banks = (self.rom.len() + 0x3FFF) / 0x4000;
        bank.min(max_banks - 1)
    }

    fn read_rom(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.rom.get(addr as usize).copied().unwrap_or(0xFF),
            0x4000..=0x7FFF => {
                let bank_base = self.active_rom_bank() * 0x4000;
                let offset = bank_base + (addr as usize - 0x4000);
                self.rom.get(offset).copied().unwrap_or(0xFF)
            }
            _ => 0xFF,
        }
    }

    fn read_ram_or_rtc(&self, addr: u16) -> u8 {
        if !self.ram_enabled {
            return 0xFF;
        }

        if self.rtc_selected {
            self.rtc_latched[(self.rtc_reg - 0x08) as usize]
        } else {
            let bank_base = self.ram_bank as usize * 0x2000;
            let offset = bank_base + (addr as usize - 0xA000);
            self.eram.get(offset).copied().unwrap_or(0xFF)
        }
    }

    fn write_ram_or_rtc(&mut self, addr: u16, value: u8) {
        if !self.ram_enabled {
            return;
        }

        if self.rtc_selected {
            self.rtc[(self.rtc_reg - 0x08) as usize] = value;
        } else {
            let bank_base = self.ram_bank as usize * 0x2000;
            let offset = bank_base + (addr as usize - 0xA000);
            if offset < self.eram.len() {
                self.eram[offset] = value;
            }
        }
    }
}
impl Cartridge for MBC3 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.read_rom(addr),
            0xA000..=0xBFFF => self.read_ram_or_rtc(addr),
            _ => 0xFF,
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = (value & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                let mut bank = value & 0x7F;
                if bank == 0 {
                    bank = 1;
                }
                self.rom_bank = bank;
            }
            0x4000..=0x5FFF => {
                if value <= 0x03 {
                    self.ram_bank = value;
                    self.rtc_selected = false;
                } else if value >= 0x08 && value <= 0x0C {
                    self.rtc_selected = true;
                    self.rtc_reg = value;
                }
            }
            0x6000..=0x7FFF => {
                if !self.latch_flag && value == 0x01 {
                    self.rtc_latched = self.rtc;
                }
                self.latch_flag = value == 0x01;
            }
            0xA000..=0xBFFF => {
                self.write_ram_or_rtc(addr, value);
            }
            _ => {}
        }
    }
}
