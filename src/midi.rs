use std::{path::Path, collections::HashMap};

use midly::{TrackEventKind, MidiMessage, MetaMessage};
use crate::Note;

pub fn parse_midi(path: &Path) -> Vec<Note> {
    let data = std::fs::read(path).unwrap();
    parse_midi_data(&data)
}


pub fn parse_midi_data(data: &[u8]) -> Vec<Note> {
    let mut notes = vec![];

    let smf = midly::Smf::parse(data).unwrap();
    dbg!(&smf.header.timing);

    let ticks_per_beat = match smf.header.timing {
        midly::Timing::Metrical(t) => t.as_int(),
        _ => unimplemented!()
    };
    let mut tick_duration = 0.0;

    for (track_nr, track) in smf.tracks.iter().enumerate() {
        println!("track {track_nr}");

        let mut tick: u64 = 0;

        let mut active = HashMap::new();

        for event in track.iter() {
            tick += event.delta.as_int() as u64;

            match event.kind {
                TrackEventKind::Midi { channel, message } => {
                    let (on, key, vel) = match message {
                        MidiMessage::NoteOn { key, vel } => (true, key.as_int(), vel.as_int()),
                        MidiMessage::NoteOff { key, vel } => (false, key.as_int(), vel.as_int()),
                        _ => continue
                    };
                    match (on, key, vel) {
                        (_, key, 0) => {
                            //println!("OFF ch: {channel}, key: {key}, vel: {vel}");
                            // find the active note
                            if let Some((start_tick, start_vel)) = active.remove(&(channel, key)) {
                                notes.push(Note {
                                    time: start_tick as f64 * tick_duration,
                                    freq: key_to_freq(key),
                                    duration: (tick - start_tick) as f64 * tick_duration,
                                    track: track_nr
                                });
                            }
                        }
                        (true, key, vel) => {
                            //println!("ON ch: {channel}, key: {key}, vel: {vel}");
                            active.insert((channel, key), (tick, vel));
                        }
                        _ => {}
                    }
                }
                TrackEventKind::Meta(meta) => {
                    match meta {
                        MetaMessage::TrackName(name) => println!("Track name: {}", String::from_utf8_lossy(name)),
                        MetaMessage::InstrumentName(name) => println!("Instrument name: {}", String::from_utf8_lossy(name)),
                        MetaMessage::Tempo(tempo) => {
                            tick_duration = 1e-6 * tempo.as_int() as f64 / ticks_per_beat as f64;
                            println!("tempo {tempo}");
                        }
                        MetaMessage::TimeSignature(num, denom, clocks_per_tick, n32_per_quarter) => {
                            println!("{num}/{denom}, clock: {clocks_per_tick}, {n32_per_quarter}/32 / quarter");
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    notes
}

fn key_to_freq(key: u8) -> f64 {
    static NOTES: [f64; 12] = [1.0, 1.0594630943592953, 1.122462048309373, 1.1892071150027212, 1.2599210498948734, 1.3348398541700346, 1.4142135623730954, 1.498307076876682, 1.5874010519682, 1.6817928305074297, 1.7817974362806794, 1.887748625363388];
    const A5: f64 = 440.0;

    let key = key + 3;
    let octave = key / 12;
    let note = key - 12 * octave;
    A5 * 2f64.powi(octave as i32 - 6) * NOTES[note as usize]
}

#[test]
fn test_key_freq() {
    assert_eq!(key_to_freq(9 + 5 * 12), 440.);
    assert_eq!(key_to_freq(9), 13.75);
}
