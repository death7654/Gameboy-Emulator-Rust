mod gameboy;

use std::time::Duration;

use gameboy::cpu::CPU;
use gameboy::gpu::GPU;
use gameboy::input::get_input;
use gameboy::ram::RAM;
use sdl2;
use sdl2::event::Event;
use sdl2::pixels::PixelFormatEnum;

const DISPLAY_HEIGHT: usize = 144;
const DISPLAY_WIDTH: usize = 160;
const SCALE: usize = 3;

const CPU_CLOCK: i32 = 4194304;

fn main() {
    let rom = std::fs::read("roms/pred.gb").unwrap();
    let ram = RAM::new(rom);
    let mut cpu = CPU::new(ram);
    let mut gpu = GPU::new();

    //games start at address 0x0100
    cpu.registers.set_pc(0x0100);
    //initialize input
    cpu.ram.write(0xFF00, 0b11111111);

    let sdl = sdl2::init().unwrap();
    let video = sdl.video().unwrap();

    let window = video
        .window(
            "Rust Game Boy",
            (DISPLAY_WIDTH * SCALE) as u32,
            (DISPLAY_HEIGHT * SCALE) as u32,
        )
        .position_centered()
        .resizable()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGB24,
            DISPLAY_WIDTH as u32,
            DISPLAY_HEIGHT as u32,
        )
        .unwrap();

    let mut event_pump = sdl.event_pump().unwrap();

    let mut frame_count = 0;

    'gameloop: loop {
        frame_count += 1;
        for evt in event_pump.poll_iter() {
            let mut value = cpu.ram.read(0xff00);
            match evt {
                Event::Quit { .. } => {
                    break 'gameloop;
                }
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if cpu.stopped {
                        cpu.stopped = false; // Resume CPU
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
            cpu.ram.write(0xff00, value);
        }
        if cpu.stopped {
            continue;
        }

       
       
        let instructions_per_frame = CPU_CLOCK / 60;
        for _ in 0..instructions_per_frame {
            let opcode = cpu.fetch();
            cpu.execute(opcode);
        }

        if frame_count % 60 == 0 {
            println!("Tilemap[0]: {}", cpu.ram.vram[0x1800]);
        }
        
        // Then render the current video memory
        gpu.render(cpu.get_vram());
        let framebuffer = gpu.get_framebuffer();
        texture.update(None, framebuffer, 160 * 3).unwrap();
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();

        std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }

    println!("Hello, world!");
}
