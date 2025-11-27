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
        // moves rom into cartridge
        // get eram size
        let eram = match rom.get(0x0149).copied().unwrap_or(0x00) {
            0x00 => 0,
            0x01 => 2 * 1024,
            0x02 => 8 * 1024,
            0x03 => 32 * 1024,
            0x04 => 128 * 1024,
            0x05 => 64 * 1024,
            _ => panic!("Unsupported ERAM size"),
        };

        let mut rtc_enabled = false;
        let mut battery = false;
        // determine
        let cartridge: Box<dyn Cartridge> = match rom[0x0147] {
            0x00 => Box::new(MBC0::new(rom)),              // ROM only
            0x01 | 0x02 => Box::new(MBC1::new(rom, eram)), // mbc1 or mbc2
            0x03 | 0x13 => {
                rtc_enabled = true;
                battery = true;
                Box::new(MBC3::new(rom, eram))
            } // mbc3 or mbc13
            other => panic!("Unsupported MBC type: {:#X}", other),
        };

        let input = Rc::new(RefCell::new(Joypad::new()));
        let shared_ram: Rc<RefCell<MMU>> =
            Rc::new(RefCell::new(MMU::new(input.clone(), cartridge)));

        let cpu = CPU::new(shared_ram.clone());
        let ppu = PPU::new(shared_ram.clone());
        let apu = Audio::new(shared_ram.clone());
        let display = Display::new();

        let emulator = EMULATOR {
            cpu,
            ppu,
            ram: shared_ram,
            input,
            apu,
            display,
            rtc_enabled,
            battery,
        };

        emulator
    }

    pub fn load_save(&mut self, file_name: &str) -> bool
    {
        let data = fs::read(file_name).unwrap_or_default();
        let mut ram = self.ram.borrow_mut();
        let mut i = 0xA000;
        for j in 0..data.len()
        {
            ram.write(i, data[j]);
            i += 1;
        }
        return true;
    }
}
