use std::cell::RefCell;
use std::rc::Rc;

use crate::gameboy::cartridge::Cartridge;
use crate::gameboy::input;

use input::Joypad;

pub struct MMU {
    cartridge: Box<dyn Cartridge>,
    pub vram: [u8; 0x2000],
    pub wram: [u8; 0x2000],
    pub rom_bank: usize, // Active ROM Bank
    pub hram: [u8; 0x7F],
    pub oam: [u8; 0xA0],
    pub io: [u8; 0x80],
    pub interrupt_enable: u8,

    // to reset the timer counter to zero if div is written to
    pub div_written: bool,
    // to check if the tiles should be recalculated
    pub vram_changed: bool,
    // to check if the ppu is active
    pub vram_blocked: bool,

    // to check if a oam is blocked during oam transfers
    pub oam_blocked: bool,
    // to check if oam dma transfer
    pub oam_dma: bool,
    pub oam_cycles: u16,
    oam_source: u16,

    pub oam_dma_wait: bool,

    // the input with shared memory
    pub joypad: Rc<RefCell<Joypad>>,
}

/*

    MMU or Memory Management Unit
        - 0x0000 -> 0x7FFF => the cartridge
        - 0x8000 -> 0x9FFF => Video RAM
        - 0xA000 -> 0xBFFF => External RAM
        - 0xC000 -> 0xDFFF => Work Ram (two banks of size 4KiB but are have adjacent addresses)
        - 0xE000 -> 0xFDFF => Echo RAM
            - Cannot be used, banned by nintendo
        - 0xFE00 -> 0xFE9F => OAM or Object Attribute Memory
        - 0xFEA0 -> 0xFEFF => Banned by Nintendo
        - 0xFF00 -> 0xFF7F => Input / Output Registers
        - 0xFF80 -> 0xFFFE => High RAM
        - 0xFFFF => Interrupt Enable Register
*/
impl MMU {
    pub fn new(joypad: Rc<RefCell<Joypad>>, cartridge: Box<dyn Cartridge>) -> Self {
        Self {
            cartridge,
            vram: [0; 0x2000],
            wram: [0; 0x2000],
            rom_bank: 1,
            hram: [0; 0x7F],
            oam: [0; 0xA0],
            io: [0; 0x80],
            interrupt_enable: 0,

            div_written: false,
            vram_changed: false,
            vram_blocked: false,
            oam_dma: false,
            oam_blocked: false,
            oam_dma_wait: false,
            oam_cycles: 0,
            oam_source: 0,
            joypad,
        }
    }

    // to read the ram's contents
    pub fn read(&mut self, address: u16) -> u8 {
        // if there is a oam_DMA transfer and the address is outside the
        // specified range then returns 0xFF
        if self.oam_dma && !(0xFF80..=0xFFFE).contains(&address) {
            return 0xFF;
        }

        match address {
            // Cartridge ROM & MMU are handled by the cartridge object
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cartridge.read(address),
            0x8000..=0x9FFF => {
                if self.vram_blocked {
                    0xFF
                } else {
                    self.vram[(address - 0x8000) as usize]
                }
            }
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize],
            0xE000..=0xFDFF => self.wram[(address - 0xE000) as usize],
            0xFE00..=0xFE9F => {
                if self.oam_dma || self.oam_blocked {
                    0xFF
                } else {
                    self.oam[(address - 0xFE00) as usize]
                }
            }
            0xFF00 => self.joypad.borrow_mut().read(),
            0xFF01..=0xFF7F => self.io[(address - 0xFF00) as usize],
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],
            0xFFFF => self.interrupt_enable,
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        if self.oam_dma && !(0xFF80..=0xFFFE).contains(&address) {
            return;
        }

        if address == 0xFF02 && value == 0x81 {
            self.handle_serial_output();
        }

        match address {
            // Cartridge ROM/MMU writes go to cartridge handler (handles MBC)
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cartridge.write(address, value),

            0x8000..=0x9FFF => {
                if !self.vram_blocked {
                    if let Some(slot) = self.vram.get_mut((address - 0x8000) as usize) {
                        *slot = value;
                        self.vram_changed = true;
                    }
                }
            }
            0xC000..=0xDFFF => {
                if let Some(slot) = self.wram.get_mut((address - 0xC000) as usize) {
                    *slot = value;
                }
            }
            0xFE00..=0xFE9F => {
                if !self.oam_blocked {
                    if let Some(slot) = self.oam.get_mut((address - 0xFE00) as usize) {
                        *slot = value;
                    }
                }
            }
            0xFF04 => {
                if let Some(slot) = self.io.get_mut(0x04) {
                    *slot = 0x00;
                    self.div_written = true;
                }
            }
            0xFF46 => {
                if let Some(slot) = self.io.get_mut((address - 0xFF00) as usize) {
                    *slot = value;
                }
                self.oam_cycles = 0;
                self.oam_dma = true;
                self.oam_source = (value as u16) << 8;
            }
            0xFF00 => self.joypad.borrow_mut().write(value),
            0xFF01..=0xFF7F => {
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
            _ => {}
        }
    }

    // functions that allows ram to be read and written to during OAM_DMA
    // mainly used to prevent CPU access
    fn read_during_dma(&mut self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cartridge.read(address),
            0x8000..=0x9FFF => {
                if self.vram_blocked {
                    0xFF
                } else {
                    self.vram[(address - 0x8000) as usize]
                }
            }
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize],
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize],
            0xFF00..=0xFF7F => self.io[(address - 0xFF00) as usize],
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],
            0xFFFF => self.interrupt_enable,
            _ => 0xFF,
        }
    }

    pub fn write_during_dma(&mut self, address: u16, value: u8) {
        match address {
            0xA000..=0xBFFF => self.cartridge.write(address, value),

            // Other memory writes remain unchanged
            0x8000..=0x9FFF => {
                //when the ppu is active the vram is blocked
                if !self.vram_blocked {
                    if let Some(slot) = self.vram.get_mut((address - 0x8000) as usize) {
                        *slot = value;
                        //vram changed to check if the vram has new data so the tiles can be recalculated
                        self.vram_changed = true;
                    }
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
                    self.div_written = true;
                }
            }
            0xFF46 => {
                if let Some(slot) = self.io.get_mut((address - 0xFF00) as usize) {
                    *slot = value;
                }
                self.oam_cycles = 0;
                self.oam_dma = true;
                self.oam_source = (value as u16) << 8;
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

    // makes sure the oam_dma is executed properly
    pub fn oam_dma_transfer(&mut self) {
        if self.oam_dma_wait {
            self.oam_dma_wait = false;
            return;
        }
        let byte = self.read_during_dma(self.oam_source + self.oam_cycles);
        self.write_during_dma(0xFE00 + self.oam_cycles, byte);
        self.oam_cycles = self.oam_cycles.wrapping_add(1);

        // the cycles are reset when a OAM_DMA
        // is initiated in the write function
        if self.oam_cycles == 164 {
            self.oam_dma = false;
        }
    }

    // mainly used for blargs testing without a display/visuals
    fn handle_serial_output(&self) {
        /*
            the serial output is found at 0xFF01
            or the second address in the IO RAM area
        */

        let data = self.io[0x01];
        print!("{}", data as char);
    }

    /*
        - special div update function,
          only able to be accessed by the timer
          as any writes to the div register wil reset it
          as per the gameboy specifications
    */
    pub fn update_div(&mut self, new: u8) {
        if let Some(slot) = self.io.get_mut((0x4) as usize) {
            *slot = new;
        }
    }
}
