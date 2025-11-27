mod gameboy;

use gameboy::EMULATOR;

use sdl2::{event::Event, pixels::PixelFormatEnum};

use crate::gameboy::cpu::CPU;

// width of a gameboy screen
const WIDTH: u32 = 160;

// height of a gameboy screen
const HEIGHT: u32 = 144;

// gameboy clock speed in normal mode
const CPU_CLOCK: u64 = 4194304;
const FRAMES: u64 = 60;

/*
Todo
- implement
    - MBC
    - Audio
    - OAM corruption bug
    - color mode
    - advance mode

 */

fn main() {
    // read a rom file relative to the location of the root directory
    let rom = std::fs::read("roms/tetris.gb").unwrap();

    // create a new emulator object and load in rom, it must be mutable
    let mut emulator = EMULATOR::new(rom);

    let mut texture = emulator
        .display
        .texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, WIDTH, HEIGHT)
        .unwrap();

    let mut event_pump = emulator.display.sdl.event_pump().unwrap();

    'gameloop: loop {
        for evt in event_pump.poll_iter() {
            match evt {
                Event::Quit { .. } => break 'gameloop,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if emulator.cpu.stopped {
                        emulator.cpu.stopped = false;
                    }
                    emulator.input.borrow_mut().set_key(key, true);

                    let mut if_reg = emulator.ram.borrow_mut().read(0xFF0F);
                    if_reg |= 1 << 4;
                    emulator.ram.borrow_mut().write(0xFF0F, if_reg);
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    emulator.input.borrow_mut().set_key(key, false);
                }
                _ => {}
            }
        }

        const CYCLES_PER_FRAME: u64 = CPU_CLOCK / FRAMES; // 4194304 Hz / 59.7 fps
        let target_cycles = emulator.cpu.cycles + CYCLES_PER_FRAME;

        while emulator.cpu.cycles < target_cycles {
            if emulator.cpu.halted {
                emulator.cpu.nop(&mut emulator.ppu);
            } else if !emulator.ram.borrow_mut().oam_dma {
                let opcode = emulator.cpu.fetch();
                emulator.cpu.execute(opcode, &mut emulator.ppu);
            } else {
                emulator.cpu.nop(&mut emulator.ppu);
            }
        }
        let framebuffer = emulator.ppu.get_framebuffer();
        texture.update(None, framebuffer, 160 * 3).unwrap();
        emulator.display.canvas.copy(&texture, None, None).unwrap();
        emulator.display.canvas.present();
    }
}
