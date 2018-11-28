use std::{path::PathBuf, time::Duration};
use vek::*;

pub mod audio_gen;
pub mod audio_mgr;

// Reexports
pub use audio::{audio_gen::AudioGen, audio_mgr::AudioMgr};

#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub relative: bool,
    pub pos: Vec3<f32>,
    pub vel: Vec3<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fade {
    pub in_duration: Duration,
    pub out_duration: Duration,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stream {
    pub buffer: u64,
    pub start_tick: Duration,
    pub duration: Duration,
    pub volume: f32,
    pub repeat: Option<()>,
    pub positional: Option<Position>,
    pub fading: Option<Fade>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Buffer {
    File(PathBuf),
    Raw(Vec<u8>),
}
