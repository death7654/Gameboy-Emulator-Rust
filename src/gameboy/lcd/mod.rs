use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::Sdl;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;
const SCALE: u32 = 3;

pub fn new() -> Result<(Sdl, Canvas<Window>), String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;

    let window = video
        .window("Rust Game Boy", WIDTH * SCALE, HEIGHT * SCALE)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;

    let canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;

    Ok((sdl, canvas))
}
