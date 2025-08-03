mod gameboy;

use gameboy::EMULATOR;

use sdl2::{event::Event, pixels::PixelFormatEnum};

// width of a gameboy screen
const WIDTH: u32 = 160;

// height of a gameboy screen
const HEIGHT: u32 = 144;

// gameboy clock speed in normal mode
const CPU_CLOCK: u32 = 4194304;

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
    let rom = std::fs::read("roms/pred.gb").unwrap();

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

                    let mut if_reg = emulator.ram.borrow().read(0xFF0F);
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

        if emulator.cpu.stopped {
            emulator.cpu.handle_interrupt(&mut emulator.ppu);
            emulator.cpu.nop(&mut emulator.ppu);
            continue 'gameloop;
        }

        if emulator.cpu.halted {
            emulator.cpu.handle_interrupt(&mut emulator.ppu);
            if emulator.cpu.halted {
                emulator.cpu.nop(&mut emulator.ppu);
                continue 'gameloop;
            }
        }

        let instructions_per_frame: u32 = CPU_CLOCK / 120;
        for _ in 0..instructions_per_frame {
            if !emulator.ram.borrow().oma_dma {
                emulator.cpu.handle_interrupt(&mut emulator.ppu);
                let opcode = emulator.cpu.fetch();
                emulator.cpu.execute(opcode, &mut emulator.ppu);
            } else {
                emulator.cpu.nop(&mut emulator.ppu);
            }
        }
        emulator.ppu.check_status();
        let framebuffer = emulator.ppu.get_framebuffer();
        texture.update(None, framebuffer, 160 * 3).unwrap();
        emulator.display.canvas.copy(&texture, None, None).unwrap();
        emulator.display.canvas.present();
    }
}
