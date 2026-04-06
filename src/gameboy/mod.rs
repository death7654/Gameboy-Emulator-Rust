use apu::Audio;
use cpu::CPU;
use display::Display;
use input::Joypad;
use mmu::MMU;
use ppu::PPU;

use core::panic;
use std::cell::RefCell;
use std::fs;
use std::rc::Rc;

use crate::gameboy::cartridge::Cartridge;
use crate::gameboy::cartridge::MBC0;
use crate::gameboy::cartridge::MBC1;
use crate::gameboy::cartridge::MBC3;

pub(crate) mod apu;
pub(crate) mod cartridge;
pub(crate) mod cpu;
pub(crate) mod display;
pub(crate) mod input;
pub(crate) mod mmu;
pub(crate) mod ppu;

pub struct EMULATOR {
    pub cpu: cpu::CPU,
    pub ppu: ppu::PPU,
    pub ram: Rc<RefCell<MMU>>,
    pub input: Rc<RefCell<Joypad>>,
    pub apu: apu::Audio,
    pub display: display::Display,

    pub rtc_enabled: bool,
    pub battery: bool,
}

impl EMULATOR {
    pub fn new(rom: Vec<u8>) -> Self {
        // Validate ROM size first
        if rom.len() < 0x150 {
            panic!("ROM too small - not a valid Game Boy ROM");
        }

        // Get ERAM size
        let eram = match rom.get(0x0149).copied().unwrap_or(0x00) {
            0x00 => 0,
            0x01 => 2 * 1024,   // Rare
            0x02 => 8 * 1024,   // Common
            0x03 => 32 * 1024,  // Common (Pokemon uses this)
            0x04 => 128 * 1024, // Rare
            0x05 => 64 * 1024,  // Rare
            _ => panic!("Unsupported ERAM size: {:#X}", rom[0x0149]),
        };

        let mut rtc_enabled = false;
        let mut battery = false;

        // Determine cartridge type with proper MBC1 variants
        let cartridge: Box<dyn Cartridge> = match rom[0x0147] {
            0x00 => Box::new(MBC0::new(rom)),       // ROM only
            0x01 => Box::new(MBC1::new(rom, eram)), // MBC1
            0x02 => {
                // MBC1+RAM
                Box::new(MBC1::new(rom, eram))
            }
            0x03 => {
                // MBC1+RAM+BATTERY
                battery = true;
                Box::new(MBC1::new(rom, eram))
            }
            0x0F => {
                // MBC3+TIMER+BATTERY
                rtc_enabled = true;
                battery = true;
                Box::new(MBC3::new(rom, eram))
            }
            0x10 => {
                // MBC3+TIMER+RAM+BATTERY
                rtc_enabled = true;
                battery = true;
                Box::new(MBC3::new(rom, eram))
            }
            0x11 => Box::new(MBC3::new(rom, eram)), // MBC3
            0x12 => Box::new(MBC3::new(rom, eram)), // MBC3+RAM
            0x13 => {
                // MBC3+RAM+BATTERY
                battery = true;
                Box::new(MBC3::new(rom, eram))
            }
            other => panic!("Unsupported MBC type: {:#X}", other),
        };

        let input = Rc::new(RefCell::new(Joypad::new()));
        let shared_ram = Rc::new(RefCell::new(MMU::new(input.clone(), cartridge)));

        let cpu = CPU::new(shared_ram.clone());
        let ppu = PPU::new(shared_ram.clone());
        let apu = Audio::new(shared_ram.clone());
        let display = Display::new();

        EMULATOR {
            cpu,
            ppu,
            ram: shared_ram,
            input,
            apu,
            display,
            rtc_enabled,
            battery,
        }
    }

    pub fn load_save(&mut self, file_name: &str) -> Result<(), String> {
        // Check if battery is supported
        if !self.battery {
            return Err("Cartridge does not support battery saves".to_string());
        }

        // Try to read the save file
        let data = match fs::read(file_name) {
            Ok(d) => d,
            Err(e) => return Err(format!("Failed to read save file: {}", e)),
        };

        // Validate save file size
        if data.is_empty() {
            return Err("Save file is empty".to_string());
        }

        // Load into cartridge RAM (0xA000-0xBFFF range)
        let mut ram = self.ram.borrow_mut();
        for (offset, &byte) in data.iter().enumerate() {
            let addr = 0xA000 + offset as u16;
            // Stop if we exceed the cartridge RAM range
            if addr >= 0xC000 {
                break;
            }
            ram.write(addr, byte);
        }

        Ok(())
    }

    pub fn save_to_file(&self, file_name: &str) -> Result<(), String> {
        if !self.battery {
            return Err("Cartridge does not support battery saves".to_string());
        }

        // Read cartridge RAM
        let mut ram = self.ram.borrow_mut();
        let mut save_data = Vec::new();

        // Determine actual RAM size from cartridge
        // Read up to 32KB (common max for Pokemon games)
        for addr in 0xA000..0xC000 {
            save_data.push(ram.read(addr));
        }

        // Write to file
        match fs::write(file_name, save_data) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to write save file: {}", e)),
        }
    }
}
