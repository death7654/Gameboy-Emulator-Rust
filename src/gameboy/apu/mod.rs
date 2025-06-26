pub struct Audio {
    audio_enabled: bool,
    channel_1: bool,
    channel_2: bool,
    channel_3: bool,
    channel_4: bool,
}
impl Audio {
    pub fn new() -> Self {
        Audio {
            audio_enabled: false,
            channel_1: false,
            channel_2: false,
            channel_3: false,
            channel_4: false,
        }
    }
}
