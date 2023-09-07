use std::path::Path;

use synth::midi::parse_midi;


fn main() {
    let mut notes = parse_midi(Path::new("Crazy_Frog_Axel.mid"));
    notes.retain(|n| n.track == 1);
    
    dbg!(notes);
}