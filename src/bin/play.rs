use std::path::Path;

use cpal::{traits::{HostTrait, DeviceTrait, StreamTrait}, SampleFormat};

use synth::midi::parse_midi;
pub use synth::synth::{Synth, Profile};

fn main() {
    let profiles = vec![
        // 0
        Profile::Simple { attack: 1.0, falloff: 1.0, colors: &[1.0, 0.8, 0.5, 0.2, 0.1]},
        
        // 1: clar1
        Profile::Simple { attack: 5.0, falloff: 20.0, colors: &[0.6, 1.0, 0.8, 0.5, 0.2]},

        // 2: bass2
        Profile::Simple { attack: 3.0, falloff: 20.0, colors: &[1.0, 0.8, 0.5, 0.3, 0.2]},

        // 3: syn
        Profile::Simple { attack: 10., falloff: 4.0, colors: &[1.0, 0.8, 0.3, 0.6, 0.2]},

        // 4: piano
        Profile::Simple { attack: 1.0, falloff: 50.0, colors: &[1.0, 0.7, 0.5, 0.4, 0.2]},
        
        // 5: clar1
        Profile::Simple { attack: 5.0, falloff: 20.0, colors: &[0.6, 1.0, 0.8, 0.5, 0.2]},

        // 6: drum
        Profile::Drum { falloff: 5.0, colors: &[2.0, 1.4, 0.8, 0.5, 0.2]},

        // 7: drum
        Profile::Drum { falloff: 10.0, colors: &[2.0, 0.8, 1.5, 0.2]},

        // 8: snare
        Profile::Drum { falloff: 10.0, colors: &[0.6, 1.0, 0.8, 0.7, 0.5, 0.4, 0.2, 0.5, 0.3, 0.1]},

        // 9: hihat
        Profile::Drum { falloff: 3.0, colors: &[0.6, 0.4, 1.0, 0.5, 0.2]},
    ];

    let mut notes = parse_midi(Path::new("Crazy_Frog_Axel.mid"));

    if let Some(t) = std::env::args().nth(1) {
        let t = t.parse().unwrap();
        notes.retain(|n| n.track == t);
    }

    let host = cpal::default_host();
    
    let device = host
        .default_output_device()
        .unwrap();

    let config = device.supported_output_configs().unwrap()
        .filter(|config| matches!(config.sample_format(), SampleFormat::F32))
        .filter(|config| config.channels() == 2)
        .next().unwrap()
        .with_max_sample_rate();

    let (mut synth, handle) = Synth::new(config.sample_rate().0, config.channels(), profiles);
    synth.queue(notes);

    let stream = device.build_output_stream(&config.into(),
        move |output, _| synth.process(output),
        |err| println!("Error building output sound stream: {}", err),
        None
    ).unwrap();
    
    stream.play().unwrap();

    handle.wait();
}
