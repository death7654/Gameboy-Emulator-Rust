use cpu::CPU;
use gpu::GPU;
use ram::RAM;
use timer::Timer;

use std::cell::RefCell;
use std::rc::Rc;

pub(crate) mod cpu;
pub(crate) mod gpu;
pub(crate) mod input;
pub(crate) mod lcd;
pub(crate) mod ram;
pub(crate) mod timer;

pub struct EMULATOR {
    pub cpu: cpu::CPU,
    pub gpu: gpu::GPU,
    pub ram: Rc<RefCell<RAM>>,
    pub timer: timer::Timer,
}

impl EMULATOR {
    pub fn new(rom: Vec<u8>) -> Self {
        let shared_ram = Rc::new(RefCell::new(RAM::new(rom)));

        let cpu = CPU::new(shared_ram.clone());
        let gpu = GPU::new(shared_ram.clone());
        let timer = Timer::new(shared_ram.clone());

        EMULATOR {
            cpu,
            gpu,
            ram: shared_ram,
            timer,
        }
    }
}
