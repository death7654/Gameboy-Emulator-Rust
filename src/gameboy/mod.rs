use cpu::CPU;
use input::Joypad;
use ppu::PPU;
use ram::RAM;

use std::cell::RefCell;
use std::rc::Rc;


pub(crate) mod cpu;
pub(crate) mod input;
pub(crate) mod lcd;
pub(crate) mod ppu;
pub(crate) mod ram;

pub struct EMULATOR {
    pub cpu: cpu::CPU,
    pub ppu: ppu::PPU,
    pub ram: Rc<RefCell<RAM>>,
    pub joypad: input::Joypad,
}

impl EMULATOR {
    pub fn new(rom: Vec<u8>) -> Self {
        let shared_ram = Rc::new(RefCell::new(RAM::new(rom)));

        let cpu = CPU::new(shared_ram.clone());
        let ppu = PPU::new(shared_ram.clone());

        let emulator = EMULATOR {
            cpu,
            ppu,
            ram: shared_ram,
            joypad: Joypad::new(),
        };

        emulator.ram.borrow_mut().write(0xFF00, 0b11111111);

        emulator
    }
}
