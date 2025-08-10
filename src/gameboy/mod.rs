use apu::Audio;
use cpu::CPU;
use display::Display;
use input::Joypad;
use ppu::PPU;
use mmu::MMU;

use std::cell::RefCell;
use std::rc::Rc;

use crate::gameboy::cartridge::Cartridge;
use crate::gameboy::cartridge::MBC0;
use crate::gameboy::cartridge::MBC1;

pub(crate) mod apu;
pub(crate) mod cpu;
pub(crate) mod display;
pub(crate) mod input;
pub(crate) mod ppu;
pub(crate) mod mmu;
pub(crate) mod cartridge;

pub struct EMULATOR {
    pub cpu: cpu::CPU,
    pub ppu: ppu::PPU,
    pub ram: Rc<RefCell<MMU>>,
    pub input: Rc<RefCell<Joypad>>,
    pub apu: apu::Audio,
    pub display: display::Display,
}

impl EMULATOR {
    pub fn new(rom: Vec<u8>) -> Self {
        // moves rom into cartridge
        let cartridge: Box<dyn Cartridge> = match rom[0x0147] {
            0x00 => Box::new(MBC0::new(rom)), // ROM only
            0x01 | 0x02 | 0x03 => Box::new(MBC1::new(rom, 0x2000)),
            other => panic!("Unsupported MBC type: {:#X}", other),
        };

        let input = Rc::new(RefCell::new(Joypad::new()));
        let shared_ram: Rc<RefCell<MMU>> = Rc::new(RefCell::new(MMU::new( input.clone(), cartridge)));

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
        };

        emulator
    }
}
