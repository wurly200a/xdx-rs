//! Render a short chord to a WAV file using a voice loaded from a .syx file.
//!
//! Usage:
//!   cargo run -p xdx-synth --example render_wav -- path/to/voice.syx output.wav
//!
//! If no .syx is given, renders with the default (INIT) voice.

use xdx_synth::FmEngine;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let voice = if args.len() >= 2 {
        let bytes = std::fs::read(&args[1]).expect("read .syx file");
        xdx_core::sysex::dx100_decode_1voice(&bytes).expect("decode voice")
    } else {
        xdx_core::dx100::Dx100Voice::default()
    };

    let out_path = args.get(2).map(|s| s.as_str()).unwrap_or("out.wav");

    println!("Voice: {}", voice.name_str());
    println!("Algorithm: {} (display {})", voice.algorithm, voice.algorithm + 1);
    println!("Writing to: {out_path}");

    const SAMPLE_RATE: u32 = 44100;
    const NOTE_DUR_S:  f32 = 1.5;
    const RELEASE_S:   f32 = 0.5;
    const NOTES: &[u8] = &[60, 64, 67]; // C major chord

    let total_samples = ((NOTE_DUR_S + RELEASE_S) * SAMPLE_RATE as f32) as usize;
    let note_off_at   = (NOTE_DUR_S * SAMPLE_RATE as f32) as usize;

    let mut engine = FmEngine::new(SAMPLE_RATE as f32);
    engine.set_voice(voice);

    for &note in NOTES {
        engine.note_on(note, 100);
    }

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(out_path, spec).expect("create wav");

    let mut buf = vec![0.0f32; 512];
    let mut pos = 0usize;

    while pos < total_samples {
        let chunk = buf.len().min(total_samples - pos);
        engine.render(&mut buf[..chunk]);

        // Fire note-offs at the right moment
        if pos < note_off_at && pos + chunk >= note_off_at {
            for &note in NOTES {
                engine.note_off(note);
            }
        }

        for &s in &buf[..chunk] {
            let clamped = s.clamp(-1.0, 1.0);
            writer.write_sample((clamped * i16::MAX as f32) as i16).unwrap();
        }
        pos += chunk;
    }

    writer.finalize().expect("finalize wav");
    println!("Done. {} samples written.", total_samples);
}
