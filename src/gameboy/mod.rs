use apu::Audio;
use cpu::CPU;
use display::Display;
use input::Joypad;
use ppu::PPU;
use ram::RAM;

use std::cell::RefCell;
use std::rc::Rc;

pub(crate) mod apu;
pub(crate) mod cpu;
pub(crate) mod display;
pub(crate) mod input;
pub(crate) mod ppu;
pub(crate) mod ram;

pub struct EMULATOR {
    pub cpu: cpu::CPU,
    pub ppu: ppu::PPU,
    pub ram: Rc<RefCell<RAM>>,
    pub input: input::Joypad,
    pub apu: apu::Audio,
    pub display: display::Display,
}

impl EMULATOR {
    pub fn new(rom: Vec<u8>) -> Self {
        let input = Joypad::new();
        let shared_ram = Rc::new(RefCell::new(RAM::new(rom, input.clone())));

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
