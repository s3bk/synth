use std::{path::Path, time::Duration};

use cpal::{traits::{HostTrait, DeviceTrait, StreamTrait}, SampleFormat};

use synth::midi::parse_midi;
pub use synth::synth::Synth;

fn main() {
    let mut notes = parse_midi(Path::new("Crazy_Frog_Axel.mid"));
    //notes.retain(|n| n.track == 1);

    let host = cpal::default_host();
    
    let device = host
        .default_output_device()
        .unwrap();

    let config = device.supported_output_configs().unwrap()
        .filter(|config| matches!(config.sample_format(), SampleFormat::F32))
        .filter(|config| config.channels() == 2)
        .next().unwrap()
        .with_max_sample_rate();

    let mut synth = Synth::new(config.sample_rate().0, config.channels());
    synth.queue(notes);

    let stream = device.build_output_stream(&config.into(),
        move |output, _| synth.process(output),
        |err| println!("Error building output sound stream: {}", err),
        None
    ).unwrap();
    
    stream.play().unwrap();

    std::thread::sleep(Duration::from_secs(60));
}
