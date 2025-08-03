//finish audio

use std::cell::RefCell;
use std::rc::Rc;

use super::ram::RAM;

#[derive(Clone)]
struct channel {
    period_counter: u8,
    wave_form: u8,
    length_timer: u16,
    volume: u8,
    pan_left: bool,
    pan_right: bool,
}
impl channel {
    fn new() -> Self {
        channel {
            period_counter: 0,
            wave_form: 0,
            length_timer: 0,
            volume: 0,
            pan_left: false,
            pan_right: false,
        }
    }
}
pub struct Audio {
    audio_enabled: bool,
    channel_1_enabled: bool,
    channel_2_enabled: bool,
    channel_3_enabled: bool,
    channel_4_enabled: bool,

    //channels
    channel_1: channel,
    channel_2: channel,
    channel_3: channel,
    channel_4: channel,

    //counter
    counter: u16,

    //volume
    ram: Rc<RefCell<RAM>>,
}
impl Audio {
    pub fn new(ram: Rc<RefCell<RAM>>) -> Self {
        let channel_struct = channel::new();
        Audio {
            audio_enabled: false,
            channel_1_enabled: false,
            channel_2_enabled: false,
            channel_3_enabled: false,
            channel_4_enabled: false,

            channel_1: channel_struct.clone(),
            channel_2: channel_struct.clone(),
            channel_3: channel_struct.clone(),
            channel_4: channel_struct.clone(),

            counter: 0,

            ram,
        }
    }
    pub fn step(&mut self) {
        self.counter += 4;
        while self.channel_1.length_timer >= 256 {
            self.channel_1.length_timer += 1;
            self.channel_2.length_timer += 1;
            self.channel_3.length_timer += 1;
            self.channel_4.length_timer += 1;
        }

        if self.channel_1.length_timer >= 64 {
            self.channel_1_enabled = false;
        }
        if self.channel_2.length_timer >= 64 {
            self.channel_2_enabled = false;
        }
        if self.channel_3.length_timer >= 256 {
            self.channel_3_enabled = false;
        }

        if self.channel_4.length_timer >= 64 {
            self.channel_4_enabled = false;
        }

        self.check_status();
    }

    fn check_status(&mut self) {
        //ff26 or audio master control
        let ff26 = self.ram.borrow().read(0xFF26);
        self.audio_enabled = ff26 & 0b1000_0000 != 0;

        if !self.audio_enabled {
            return;
        }

        self.channel_4_enabled = ff26 & 0b0000_1000 != 0;
        self.channel_3_enabled = ff26 & 0b0000_0100 != 0;
        self.channel_2_enabled = ff26 & 0b0000_0010 != 0;
        self.channel_1_enabled = ff26 & 0b0000_0001 != 0;

        // ff25 determines the panning of a sound per channel
        let ff25 = self.ram.borrow().read(0xFF25);

        self.channel_4.pan_left = ff25 & 0b1000_0000 != 0;
        self.channel_3.pan_left = ff25 & 0b0100_0000 != 0;
        self.channel_2.pan_left = ff25 & 0b0010_0000 != 0;
        self.channel_1.pan_left = ff25 & 0b0001_0000 != 0;

        self.channel_4.pan_right = ff25 & 0b0000_1000 != 0;
        self.channel_3.pan_right = ff25 & 0b0000_0100 != 0;
        self.channel_2.pan_right = ff25 & 0b0000_0010 != 0;
        self.channel_1.pan_right = ff25 & 0b0000_0001 != 0;
    }
}
