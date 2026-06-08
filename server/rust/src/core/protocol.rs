use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Command {
    Key {
        key_code: u16,
        pressed: bool,
    },
    MouseRelative {
        dx: i32,
        dy: i32,
    },
    MouseAbsolute {
        x: i32,
        y: i32,
        max_x: i32,
        max_y: i32,
    },
    MouseClick {
        button: u16,
        pressed: bool,
    },
    MouseScroll {
        steps: i32,
    },
    StreamConfig {
        backpressure: bool,
        codec: String,
        target_fps: u32,
    },
}
