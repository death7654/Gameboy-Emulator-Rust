mod gameboy;

use gameboy::input::get_input;
use gameboy::lcd;
use gameboy::ram::RAM;
use gameboy::EMULATOR;

use sdl2;
use sdl2::event::Event;
use sdl2::pixels::PixelFormatEnum;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

const CPU_CLOCK: u32 = 4194304;

/*
Tests Passed
01-special.gb
03-op sp,hl.gb
04-op r,imm.gb
05-op rp.gb
06-ld r,r.gb
07-jr,jp,call,ret,rst.gb
08-misc instrs.gb
09-op r,r.gb
10-bit ops.gb
11-op a,(hl).gb

 */

fn main() {
    //let rom = std::fs::read("roms/drmario.gb").unwrap();
    //let rom = std::fs::read("roms/test_roms/test_cart.gb").unwrap();
    let rom = std::fs::read("roms/test_roms/blargg-test/02-interrupts.gb").unwrap();
    //let rom = std::fs::read("roms/test_roms/cpu.gb").unwrap();
    let mut emulator = EMULATOR::new(rom);

    //games start at address 0x0100
    emulator.cpu.registers.set_pc(0x0100);

    //intialize input
    // emulator.ram.borrow_mut().write(0xFF00, 0b11111111);

    //intialize window
    let (sdl, mut canvas) = match lcd::new() {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Failed to initialize SDL2: {}", e);
            return;
        }
    };
    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, WIDTH, HEIGHT)
        .unwrap();
    let mut event_pump = sdl.event_pump().unwrap();

    'gameloop: loop {
        // Input Handling
        for evt in event_pump.poll_iter() {
            let current = emulator.ram.borrow().read(0xFF00);
            match evt {
                Event::Quit { .. } => break 'gameloop,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if emulator.cpu.stopped {
                        emulator.cpu.stopped = false;
                        println!("Key {:?} pressed. Resuming CPU...", key);
                    }

                    let updated = get_input(key, true, current);
                    emulator.ram.borrow_mut().write(0xFF00, updated);

                    let mut if_reg = emulator.ram.borrow().read(0xFF0F);
                    if_reg |= 1 << 4;
                    emulator.ram.borrow_mut().write(0xFF0F, if_reg);
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    let updated = get_input(key, false, current);
                    emulator.ram.borrow_mut().write(0xFF00, updated);
                }
                _ => {}
            }
        }

        if emulator.cpu.stopped || emulator.cpu.halted {
            emulator.cpu.handle_interrupt();
            if emulator.cpu.stopped {
                // No pending interrupts; simulate idle cycles
                emulator.cpu.cycles += 4;
                continue;
            }
        }
        

        // CPU Execution
        let instructions_per_frame = CPU_CLOCK / 60;
        for _ in 0..instructions_per_frame {
            let opcode = emulator.cpu.fetch();
            emulator.cpu.execute(opcode);
            if emulator.cpu.ime && (emulator.ram.borrow().read(0xFF0F) != 0) {
                println!(
                    "Interrupt Triggered! Flags: {:02X}",
                    emulator.ram.borrow().read(0xFF0F)
                );
                emulator.cpu.handle_interrupt();
            }
        }

        // Render current video memory
        emulator.gpu.render();
        let fb = emulator.gpu.get_framebuffer();
        texture.update(None, fb, 160 * 3).unwrap();
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }

    println!("Game loop exited.");
}

pub fn enable_test_pattern(ram: &mut RAM) {
    // Set LCD control to turn on display
    ram.io[0x40] = 0x91; // LCDC: LCD enabled, BG enabled

    // Set background palette
    ram.io[0x47] = 0xFC; // BGP (white, light gray, dark gray, black)

    // Clear VRAM first
    for byte in ram.vram.iter_mut() {
        *byte = 0;
    }

    // Fill one tile with a checkerboard pattern
    for row in 0..8 {
        if row % 2 == 0 {
            ram.vram[0x0000 + row * 2] = 0b10101010;
            ram.vram[0x0000 + row * 2 + 1] = 0;
        } else {
            ram.vram[0x0000 + row * 2] = 0b01010101;
            ram.vram[0x0000 + row * 2 + 1] = 0;
        }
    }

    // Fill background tilemap to point to tile 0
    for i in 0..(32 * 32) {
        ram.vram[0x1800 + i] = 0; // Tile 0
    }
}
