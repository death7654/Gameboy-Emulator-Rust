mod gameboy;

use gameboy::input::get_input;
use gameboy::lcd;
use gameboy::EMULATOR;

use sdl2;
use sdl2::event::Event;
use sdl2::pixels::PixelFormatEnum;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

const CPU_CLOCK: u32 = 4194304;

fn main() {
    let rom = std::fs::read("roms/test_cart.gb").unwrap();
    let mut emulator = EMULATOR::new(rom);

    //games start at address 0x0100
    emulator.cpu.registers.set_pc(0x0100);

    //intialize input
    emulator.cpu.ram.borrow_mut().write(0xFF00, 0b11111111);

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

    let mut frame_count = 0;

    'gameloop: loop {
        frame_count += 1;
        for evt in event_pump.poll_iter() {
            let mut value = emulator.cpu.ram.borrow().read(0xff00);
            match evt {
                Event::Quit { .. } => {
                    break 'gameloop;
                }
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if emulator.cpu.stopped {
                        emulator.cpu.stopped = false; // Resume CPU
                        println!("Key {:?} pressed. Resuming CPU...", key);
                    }
                    value &= get_input(key, true);
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    value |= get_input(key, false);
                }
                _ => (),
            }
            emulator.cpu.ram.borrow_mut().write(0xff00, value);
        }
        if emulator.cpu.stopped {
            continue;
        }

        let instructions_per_frame = CPU_CLOCK / 60;
        for _ in 0..instructions_per_frame {
            let opcode = emulator.cpu.fetch();
            emulator.cpu.execute(opcode);
        }

        // Then render the current video memory
        emulator.gpu.render();
        let fb = emulator.gpu.get_framebuffer();
        texture.update(None, fb, 160 * 3).unwrap();
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }

    println!("Hello, world!");
}
