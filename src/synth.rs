use std::{collections::VecDeque, sync::{Condvar, Arc, mpsc::{Receiver, channel, Sender}}};

use crate::Note;


enum Msg {
    Done
}

pub struct SynthHandle {
    rx: Receiver<Msg>,
}
impl SynthHandle {
    pub fn wait(&self) {
        self.rx.recv().unwrap();
    }
}

pub struct Synth {
    channels: u16,
    sample_rate: u32,
    queue: VecDeque<Note>,
    playing: Vec<Voice>,
    time: f64,
    sample_duration: f64,
    tx: Sender<Msg>,
    profiles: Vec<Profile>,
}
impl Synth {
    pub fn new(sample_rate: u32, channels: u16, profiles: Vec<Profile>) -> (Self, SynthHandle) {
        let (tx, rx) = channel();

        let synth = Synth {
            channels,
            sample_rate,
            queue: Default::default(),
            playing: Default::default(),
            time: 0.0,
            sample_duration: 1.0 / sample_rate as f64,
            tx,
            profiles
        };
        let handle = SynthHandle { rx };
        (synth, handle)
    }
    fn process_part(&mut self, output: &mut [f32]) {
        output.fill(0.0);
        if self.playing.len() == 0 {
            return;
        }
        // println!("{} voices for {} samples", self.playing.len(), output.len() / self.channels as usize);
        for voice in self.playing.iter_mut() {
            match voice {
                Voice::Simple(v) => {
                    for out in output.chunks_exact_mut(self.channels as usize) {
                        for c in out.iter_mut() {
                            *c += v.sample();
                        }
                    }
                }
                Voice::Drum(v) => {
                    for out in output.chunks_exact_mut(self.channels as usize) {
                        for c in out.iter_mut() {
                            *c += v.sample();
                        }
                    }
                }
            }
        }
        self.playing.retain(|v| match v {
            Voice::Simple(v) => !v.stopped(),
            Voice::Drum(v) => !v.stopped(),
        });
    }
    pub fn process(&mut self, mut remaining: &mut [f32]) {
        let samples = remaining.len() / self.channels as usize;
        let end_time = self.time + samples as f64 * self.sample_duration;

        while let Some(next) = self.queue.front() {
            //dbg!(next);
            if next.time < end_time {
                let note = self.queue.pop_front().unwrap();
                let profile = &self.profiles[note.track % self.profiles.len()];
                let v = profile.voice(note.freq, self.sample_duration, 0.1, note.duration);
                self.playing.push(v);

                let samples = ((note.time - self.time) * self.sample_rate as f64) as usize;
                let idx = samples * self.channels as usize;
                let (first, remaining2) = remaining.split_at_mut(idx);
                if first.len() > 0 {
                    self.process_part(first);
                }
                remaining = remaining2;
                self.time += samples as f64 * self.sample_duration;
            } else {
                break;
            }
        }
        if remaining.len() > 0 {
            self.process_part(remaining);
            self.time += (remaining.len() / self.channels as usize) as f64 * self.sample_duration;
        }
        if self.queue.len() == 0 {
            self.tx.send(Msg::Done).unwrap();
        }
    }
    pub fn queue(&mut self, mut notes: Vec<Note>) {
        notes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        self.queue.extend(notes);
        if let Some(first) = self.queue.front() {
            self.time = first.time;
        }
    }
}

enum Voice {
    Simple(SimpleVoice),
    Drum(DrumVoice),
}

pub enum Profile {
    Simple { colors: &'static [f32], attack: f32, falloff: f32 },
    Drum { colors: &'static [f32], falloff: f32 },
}
impl Profile {
    fn voice(&self, freq: f64, sample_duration: f64, amplitude: f64, duration: f64) -> Voice {
        match *self {
            Profile::Simple { colors, attack, falloff } => Voice::Simple(SimpleVoice::new(freq, sample_duration, amplitude, duration, attack, falloff, colors)),
            Profile::Drum { colors, falloff } => Voice::Drum(DrumVoice::new(freq, sample_duration, amplitude, duration, falloff, colors))
        }
    }
}

struct SimpleVoice {
    phase: f32,
    phase_step: f32,
    amplitude: f32,
    target_amplitide: f32,
    samples_remaining: i64,
    attack: f32,
    falloff: f32,
    colors: &'static [f32],
}
impl SimpleVoice {
    pub fn new(freq: f64, sample_duration: f64, amplitude: f64, duration: f64, attack: f32, falloff: f32, colors: &'static [f32]) -> Self {
        let phase_step = (sample_duration * freq) as f32;
        SimpleVoice {
            phase: 0.0,
            phase_step,
            amplitude: 0.0,
            target_amplitide: (100. * amplitude as f32 / freq as f32).min(0.5),
            samples_remaining: ((duration + 3.0 * falloff as f64 / freq) / sample_duration) as i64,
            falloff: (phase_step / falloff).min(1.0),
            attack: (phase_step / attack).min(1.0),
            colors,
        }
    }
    fn stop(&mut self) {
        self.target_amplitide = 0.0;
    }
    pub fn sample(&mut self) -> f32 {
        use std::f32::consts::PI;

        if self.samples_remaining == 0 {
            self.stop();
        }

        let f = if self.samples_remaining > 0 {
            self.attack
        } else {
            self.falloff
        };
        self.amplitude = f * self.target_amplitide + (1.0 - f) * self.amplitude;
        self.phase = (self.phase + self.phase_step).fract();

        self.samples_remaining -= 1;

        self.colors.iter().enumerate().map(|(i, &c)| 
            (i as f32 * 2.0 * PI * self.phase as f32).sin() * c
        ).sum::<f32>()* self.amplitude 
    }
    pub fn stopped(&self) -> bool {
        self.samples_remaining <= 0
    }
}
struct DrumVoice {
    phase: f32,
    phase_step: f32,
    amplitude: f32,
    falloff: f32,
    colors: &'static [f32],
    samples_remaining: i64,
}
impl DrumVoice {
    pub fn new(freq: f64, sample_duration: f64, amplitude: f64, duration: f64, falloff: f32, colors: &'static [f32]) -> Self {
        let phase_step = (sample_duration * freq) as f32;
        DrumVoice {
            phase: 0.0,
            phase_step,
            amplitude: (100. * amplitude as f32 / freq as f32).min(0.5),
            falloff: (phase_step / falloff).min(1.0),
            samples_remaining: ((falloff as f64 / freq * 5.0) / sample_duration) as i64,
            colors,
        }
    }
    pub fn sample(&mut self) -> f32 {
        use std::f32::consts::PI;


        let f = self.falloff;
        self.amplitude = (1.0 - f) * self.amplitude;
        self.phase = (self.phase + self.phase_step).fract();

        self.samples_remaining -= 1;

        self.colors.iter().enumerate().map(|(i, &c)| 
            (i as f32 * 2.0 * PI * self.phase as f32).sin() * c
        ).sum::<f32>()* self.amplitude 
    }
    pub fn stopped(&self) -> bool {
        self.samples_remaining <= 0
    }
}