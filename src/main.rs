mod gameboy;

use gameboy::ram::RAM;

const DISPLAY_HEIGHT: usize = 144;
const DISPLAY_WIDTH: usize = 160;

fn main() {
    //creating ram
    let ram = RAM::new();
    println!("Hello, world!");
}
