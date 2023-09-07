use std::collections::VecDeque;

use crate::Note;

static COLORS: &[&[f32]] = &[
    &[1.0, 0.8, 0.5, 0.2, 0.1],
    &[0.6, 1.0, 0.8, 0.5, 0.2],
    &[1.0, 0.2, 0.3, 0.6, 0.2],
];

pub struct Synth {
    channels: u16,
    sample_rate: u32,
    queue: VecDeque<Note>,
    playing: Vec<Voice>,
    time: f64,
    sample_duration: f64,
}
impl Synth {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Synth {
            channels,
            sample_rate,
            queue: Default::default(),
            playing: Default::default(),
            time: 0.0,
            sample_duration: 1.0 / sample_rate as f64,
        }
    }
    fn process_part(&mut self, output: &mut [f32]) {
        output.fill(0.0);
        if self.playing.len() == 0 {
            return;
        }
        println!("{} voices for {} samples", self.playing.len(), output.len() / self.channels as usize);
        for voice in self.playing.iter_mut() {
            match voice {
                Voice::Simple(v) => {
                    for out in output.chunks_exact_mut(self.channels as usize) {
                        for c in out.iter_mut() {
                            *c += v.sample();
                        }
                    }
                }
            }
        }
        self.playing.retain(|v| match v {
            Voice::Simple(v) => !v.stopped()
        });
    }
    pub fn process(&mut self, mut remaining: &mut [f32]) {
        let samples = remaining.len() / self.channels as usize;
        let end_time = self.time + samples as f64 * self.sample_duration;

        while let Some(next) = self.queue.front() {
            //dbg!(next);
            if next.time < end_time {
                let note = self.queue.pop_front().unwrap();
                let color = COLORS[note.track % COLORS.len()];
                let v = SimpleVoice::new(note.freq, self.sample_duration, 0.1, note.duration, color);
                self.playing.push(Voice::Simple(v));

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
    }
    pub fn queue(&mut self, mut notes: Vec<Note>) {
        notes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        self.queue.extend(notes);
    }
}

enum Voice {
    Simple(SimpleVoice)
}

struct SimpleVoice {
    phase: f32,
    phase_step: f32,
    amplitude: f32,
    target_amplitide: f32,
    samples_remaining: i64,
    colors: &'static [f32],
}
impl SimpleVoice {
    pub fn new(freq: f64, sample_duration: f64, amplitude: f64, duration: f64, colors: &'static [f32]) -> Self {
        SimpleVoice {
            phase: 0.0,
            phase_step: (sample_duration * freq) as f32,
            amplitude: amplitude as f32,
            target_amplitide: amplitude as f32,
            samples_remaining: (duration / sample_duration) as i64,
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

        self.amplitude = 0.02 * self.target_amplitide + 0.98 * self.amplitude;
        self.phase = (self.phase + self.phase_step).fract();

        self.samples_remaining -= 1;

        self.colors.iter().enumerate().map(|(i, &c)| 
            (i as f32 * 2.0 * PI * self.phase as f32).sin() * c
        ).sum::<f32>()* self.amplitude 
    }
    pub fn stopped(&self) -> bool {
        self.samples_remaining <= 0 && self.amplitude < 0.001
    }
}