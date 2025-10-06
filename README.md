# Gameboy-Emulator-Rust


## Project Screenshots
<img width="361" height="348" alt="image" src="https://github.com/user-attachments/assets/08dae415-ac24-44ac-bbd7-25ae78fe6db5" />
<img width="361" height="348" alt="image" src="https://github.com/user-attachments/assets/f40fa8cd-d219-47f6-ac14-ad67e640674a" />
<img width="361" height="348" alt="image" src="https://github.com/user-attachments/assets/957c915d-8e76-4fb1-afd8-1f6460397f24" />
<img width="361" height="348" alt="image" src="https://github.com/user-attachments/assets/6a588a05-1867-455b-b430-a344ea8584e6" />

## About
This project is a personal way for me to learn about computer systems and improve my programming skills. It is a software simulation of the Game Boy that aims to replicate the hardware behavior as accurately as possible.

## Prerequisites 
  1. Install Rust from their official [website](https://rust-lang.org/tools/install/)
     
## Running the Emulator
  1. Clone the project using the following command ```git clone https://github.com/death7654/Gameboy-Emulator-Rust.git```
  2. Open a terminal, and  ```cd``` to the project
  3. Run ```cargo build```
  4. The emulator depends on SDL2 for rendering. To ensure it runs correctly, copy the contents of the `dependencies` folder into the `target/debug` folder:
     - Linux/macOS
         - ```cp dependencies/* target/debug/```
     - Windows
        - ```Copy-Item dependencies\* target\debug\```
  5. In main.rs, change the first line in the main function to the path to your rom
     - ```let rom = std::fs::read("C:/path/to/rom.gb").unwrap();```
  6. Run ```cargo run```

## Features not implemented
  1. Audio
  2. Save game state
  3. Game Boy Color Support
  4. Game Boy Advance Support
  5. High Level Useability Tools







