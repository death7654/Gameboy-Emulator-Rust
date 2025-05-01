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
    //let rom = std::fs::read("roms/tetris.gb").unwrap();
    //let rom = std::fs::read("roms/test_roms/cpu.gb").unwrap();
    let rom = std::fs::read("roms/test_roms/blargg-test/2.gb").unwrap();

    //let rom = std::fs::read("roms/test_roms/mooneye/acceptance/timer/tim00.gb").unwrap();
    //let rom = std::fs::read("roms/test_roms/mooneye/acceptance/timer/tima_write_reloading.gb").unwrap();

    let mut emulator = EMULATOR::new(rom);

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

    //enable_test_pattern(&mut emulator.ram.borrow_mut());

    emulator.cpu.log_cpu_state();

    'gameloop: loop {
        // 1. Input Handling
        for evt in event_pump.poll_iter() {
            // Read current joypad state from 0xFF00.
            let current = emulator.ram.borrow().read(0xFF00);
            match evt {
                Event::Quit { .. } => break 'gameloop,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    // If the CPU was stopped (waiting for input), resume it.
                    if emulator.cpu.stopped {
                        emulator.cpu.stopped = false;
                        // Uncomment the next line for debugging:
                        println!("Key {:?} pressed. Resuming CPU...", key);
                    }
                    // Update joypad state for key press.
                    let updated = get_input(key, true, current);
                    emulator.ram.borrow_mut().write(0xFF00, updated);

                    // Set the joypad interrupt flag (bit 4 of IF at 0xFF0F) to trigger an interrupt
                    let mut if_reg = emulator.ram.borrow().read(0xFF0F);
                    if_reg |= 1 << 4;
                    emulator.ram.borrow_mut().write(0xFF0F, if_reg);
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    // Update joypad state for key release.
                    let updated = get_input(key, false, current);
                    emulator.ram.borrow_mut().write(0xFF00, updated);
                }
                _ => {}
            }
        }

        // 2. Handle CPU Stopped or Halted State
        if emulator.cpu.stopped || emulator.cpu.halted {
            emulator.cpu.handle_interrupt();
            // If still stopped, simulate idle cycles before processing the next frame.
            if emulator.cpu.stopped {
                emulator.cpu.cycles += 4;
                emulator.timer.timer(4);
                continue 'gameloop;
            }
        }

        // 3. CPU Instruction Execution
        // Execute a fixed number of instructions per frame.
        let instructions_per_frame: u32 = CPU_CLOCK / 120;
        for _ in 0..instructions_per_frame {
            let pre_exec_cycles = emulator.cpu.cycles;
            emulator.cpu.handle_interrupt();

            let opcode = emulator.cpu.fetch();
            emulator.cpu.execute(opcode);

            //emulator.cpu.log_cpu_state();
            let after_exec_cycles = emulator.cpu.cycles;

            emulator
                .timer
                .timer((after_exec_cycles - pre_exec_cycles) as u16);
        }

        // 4. Render the Video Output
        emulator.gpu.render();
        let fb = emulator.gpu.get_framebuffer();
        texture.update(None, fb, 160 * 3).unwrap();
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }

    //println!("Game loop exited.");
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

#[test]
fn test_halt_exits_on_timer_interrupt() {
    use crate::gameboy::cpu::CPU;
    use crate::gameboy::ram::RAM;
    use std::cell::RefCell;
    use std::rc::Rc;

    let rom = vec![0; 0x8000];
    let ram = Rc::new(RefCell::new(RAM::new(rom)));
    let mut cpu = CPU::new(ram.clone());

    ram.borrow_mut().write(0xFF07, 0x05);

    ram.borrow_mut().write(0xFF05, 0x00);

    ram.borrow_mut().write(0xFF0F, 0x00);

    cpu.halted = true;

    println!("Halt: {}", cpu.halted);

    let mut if_reg = ram.borrow().read(0xFF0F);

    println!("if reg: {:08b}", if_reg);
    if_reg |= 1 << 2; // Set bit 2 (Timer interrupt)
    ram.borrow_mut().write(0xFF0F, if_reg);

    cpu.handle_interrupt();
    println!("Halt: {}", cpu.halted);
    println!("if reg: {:08b}", if_reg);

    assert!(
        !cpu.halted,
        "CPU should exit HALT when timer interrupt occurs"
    );

    let if_reg = ram.borrow().read(0xFF0F);
    assert!(
        (if_reg & (1 << 2)) != 0,
        "Timer interrupt should be pending"
    );
}
