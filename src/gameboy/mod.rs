use cpu::CPU;
use input::Joypad;
use ppu::PPU;
use ram::RAM;
use apu::Audio;
use display::Display;

use std::cell::RefCell;
use std::rc::Rc;


pub(crate) mod cpu;
pub(crate) mod input;
pub(crate) mod ppu;
pub(crate) mod ram;
pub(crate) mod apu;
pub(crate) mod display;

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
        let shared_ram = Rc::new(RefCell::new(RAM::new(rom)));


        let cpu = CPU::new(shared_ram.clone());
        let ppu = PPU::new(shared_ram.clone());
        let input= Joypad::new();
        let apu = Audio::new();
        let display = Display::new();

        let emulator = EMULATOR {
            cpu,
            ppu,
            ram: shared_ram,
            input,
            apu,
            display
        };


        emulator
    }
}
