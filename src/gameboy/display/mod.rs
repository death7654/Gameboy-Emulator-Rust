/*
Todo:
- move everything display related into this
 */

use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::Sdl;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;
const SCALE: u32 = 3;

pub struct Display {
    pub canvas: Canvas<Window>,
    pub sdl: Sdl,
    pub texture_creator: TextureCreator<WindowContext>,
}

impl Display {
    pub fn new() -> Self {
        let sdl = sdl2::init().unwrap();
        let video = sdl.video().unwrap();

        // creates a window with its proper scaling
        let window = video
            .window("Rust Game Boy", WIDTH * SCALE, HEIGHT * SCALE)
            .position_centered()
            .resizable()
            .build()
            .unwrap();

        //creates a canvas
        let canvas = window
            .into_canvas()
            .present_vsync()
            .build()
            .unwrap();

        let texture_creator = canvas.texture_creator();
        

        Self {
            canvas,
            sdl,
            texture_creator,
        }
    }
   
}
