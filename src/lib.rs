
pub mod synth;
pub mod midi;

#[derive(Debug)]
pub struct Note {
    pub time: f64,
    pub freq: f64,
    pub duration: f64,
    pub track: usize,
}
