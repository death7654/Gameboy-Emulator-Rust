mod gameboy;

use gameboy::EMULATOR;
use sdl2::{event::Event, pixels::PixelFormatEnum};

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

// Gameboy runs at ~4.194304 MHz, displaying ~59.7 fps
const CPU_CLOCK: u64 = 4194304;
const FRAMES_PER_SECOND: u64 = 60;
const CYCLES_PER_FRAME: u64 = CPU_CLOCK / FRAMES_PER_SECOND; // ~69905 cycles per frame

fn main() {
    let rom = std::fs::read("roms/pred.gb").unwrap();
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

        let start_cycles = emulator.cpu.cycles;
        let target_cycles = start_cycles + CYCLES_PER_FRAME;

        // Execute instructions until we've completed a frame's worth of cycles
        while emulator.cpu.cycles < target_cycles {
            // Handle OAM DMA transfer
            if emulator.ram.borrow().oam_dma {
                emulator.cpu.tick(&mut emulator.ppu);
                continue;
            }

            // Handle interrupts 
            emulator.cpu.handle_interrupt(&mut emulator.ppu);

            // If halted or stopped, just tick without executing
            if emulator.cpu.halted || emulator.cpu.stopped {
                emulator.cpu.tick(&mut emulator.ppu);
                continue;
            }

            // Normal instruction execution
            let opcode = emulator.cpu.fetch();
            emulator.cpu.execute(opcode, &mut emulator.ppu);
        }

        // Render the completed frame
        let framebuffer = emulator.ppu.get_framebuffer();
        texture.update(None, framebuffer, 160 * 3).unwrap();
        emulator.display.canvas.copy(&texture, None, None).unwrap();
        emulator.display.canvas.present();
    }
}