mod gameboy;

use gameboy::ram::RAM;
use gameboy::cpu::CPU;
use sdl2;

const DISPLAY_HEIGHT: usize = 144;
const DISPLAY_WIDTH: usize = 160;

fn main() {
    let rom = std::fs::read("roms/pred.gb").unwrap();
    let mut ram = RAM::new(rom);
    let mut cpu = CPU::new(&mut ram);
    cpu.registers.set_pc(0x100);

    loop {
        let opcode = cpu.fetch();
        cpu.execute(opcode);
    }

    println!("Hello, world!");
}
