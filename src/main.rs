mod gameboy;

use std::time::Duration;

use gameboy::cpu::CPU;
use gameboy::gpu::GPU;
use gameboy::ram::RAM;
use sdl2;
use sdl2::pixels::PixelFormatEnum;


const DISPLAY_HEIGHT: usize = 144;
const DISPLAY_WIDTH: usize = 160;
const SCALE: usize = 3;

fn main() {
    let rom = std::fs::read("roms/cpu.gb").unwrap();
    let mut ram = RAM::new(rom);
    let mut cpu = CPU::new(ram);
    let mut gpu = GPU::new();

    //games start at address 0x0100
    cpu.registers.set_pc(0x0100);


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

loop {
    // Run 1000–5000 instructions per frame
    for _ in 0..50000 {
        let opcode = cpu.fetch();

        cpu.execute(opcode);
    }
    frame_count += 1;

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

