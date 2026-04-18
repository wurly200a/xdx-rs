use xdx_midi::MidiManager;

fn main() {
    println!("MIDI IN ports:");
    for p in MidiManager::list_in_ports() {
        println!("  {:?}", p);
    }
    println!("MIDI OUT ports:");
    for p in MidiManager::list_out_ports() {
        println!("  {:?}", p);
    }
}
