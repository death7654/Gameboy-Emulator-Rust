mod gameboy;

use gameboy::lcd;
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
    //let rom = std::fs::read("roms/test_roms/blargg/halt_bug.gb").unwrap();
    let rom = std::fs::read("roms/test_roms/blargg/cpu_instrs/individual/02-interrupts.gb").unwrap();
    //let rom = std::fs::read("roms/test_roms/blargg/oam_bug/oam_bug.gb").unwrap();

    //let rom = std::fs::read("roms/test_roms/mooneye-test-suite/acceptance/timer/tim11.gb").unwrap();
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

    //emulator.cpu.log_cpu_state();

    'gameloop: loop {
        // 1. Input Handling
        for evt in event_pump.poll_iter() {
            // Read current joypad state from 0xFF00.

            match evt {
                Event::Quit { .. } => break 'gameloop,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if emulator.cpu.stopped {
                        emulator.cpu.stopped = false;
                        println!("Key {:?} pressed. Resuming CPU...", key);
                    }
                    emulator.joypad.set_key(key, true);
                    let updated = emulator.joypad.read();
                    emulator.ram.borrow_mut().write(0xFF00, updated);

                    let mut if_reg = emulator.ram.borrow().read(0xFF0F);
                    if_reg |= 1 << 4;
                    emulator.ram.borrow_mut().write(0xFF0F, if_reg);
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    emulator.joypad.set_key(key, false);
                    let updated = emulator.joypad.read();
                    emulator.ram.borrow_mut().write(0xFF00, updated);
                }
                _ => {}
            }
        }
        // let ram_data = emulator.ram.borrow().read(0xFF00);
        // println!("current: {}", ram_data);

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
        let instructions_per_frame: u32 = CPU_CLOCK / 60;
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
