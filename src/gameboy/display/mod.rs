use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Canvas, Texture, TextureCreator};
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

        let window = video
            .window("Rust Game Boy", WIDTH * SCALE, HEIGHT * SCALE)
            .position_centered()
            .resizable()
            .build()
            .unwrap();

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

    /// You can call this whenever you need a texture.
    pub fn create_texture(&self) -> sdl2::render::Texture {
        self.texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, WIDTH, HEIGHT)
            .expect("Failed to create texture")
    }
    pub fn present_display(mut self, texture:&Texture<'_>)
    {
        self.canvas.clear();
        self.canvas.copy(&texture, None, None).unwrap();
        self.canvas.present();
    }
}
