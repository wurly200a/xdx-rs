//! Render a voice to a WAV file using a .syx file.
//!
//! Usage:
//!   cargo run -p xdx-synth --example render_wav -- [voice.syx [out.wav [midi_note]]]
//!
//! Defaults: default voice, out.wav, MIDI note 69 (A4 = 440 Hz)

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
    let midi_note: u8 = args.get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(69); // A4 = 440 Hz

    dump_voice(&voice, midi_note);
    println!("Writing to: {out_path}");

    const SAMPLE_RATE: u32 = 44100;
    const NOTE_DUR_S:  f32 = 2.0;
    const RELEASE_S:   f32 = 1.0;

    let total_samples = ((NOTE_DUR_S + RELEASE_S) * SAMPLE_RATE as f32) as usize;
    let note_off_at   = (NOTE_DUR_S * SAMPLE_RATE as f32) as usize;

    let mut engine = FmEngine::new(SAMPLE_RATE as f32);
    engine.set_voice(voice);
    engine.note_on(midi_note, 100);

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

        if pos < note_off_at && pos + chunk >= note_off_at {
            engine.note_off(midi_note);
        }

        for &s in &buf[..chunk] {
            let clamped = s.clamp(-1.0, 1.0);
            writer.write_sample((clamped * i16::MAX as f32) as i16).unwrap();
        }
        pos += chunk;
    }

    writer.finalize().expect("finalize wav");
    println!("Done. {:.1}s + {:.1}s release = {} samples.",
        NOTE_DUR_S, RELEASE_S, total_samples);
}

fn dump_voice(v: &xdx_core::dx100::Dx100Voice, midi_note: u8) {
    const FREQ_RATIOS: [f32; 64] = [
        0.50, 0.71, 1.00, 1.41, 1.50, 1.73, 2.00, 2.50,
        2.83, 3.00, 3.54, 4.00, 4.24, 4.50, 5.00, 5.66,
        6.00, 6.36, 7.00, 7.07, 8.00, 8.49, 9.00, 9.50,
        10.00, 10.99, 11.00, 12.00, 12.73, 13.00, 14.00, 14.14,
        15.00, 17.00, 18.00, 19.00, 20.00, 21.00, 22.00, 24.00,
        25.00, 26.00, 27.00, 28.00, 29.00, 30.00, 32.00, 33.00,
        34.00, 35.00, 36.00, 38.00, 40.00, 42.00, 44.00, 46.00,
        48.00, 50.00, 52.00, 54.00, 56.00, 58.00, 60.00, 64.00,
    ];
    let base_hz = 440.0 * 2.0_f32.powf((midi_note as f32 - 69.0
        + v.transpose as f32 - 24.0) / 12.0);

    println!("=== Voice: \"{}\" ===", v.name_str());
    println!("Algorithm: {} (display {})  Feedback: {}  Transpose: {} (center=24)",
        v.algorithm, v.algorithm + 1, v.feedback, v.transpose);
    println!("MIDI note: {} → base {:.1} Hz", midi_note, base_hz);
    println!();
    println!("{:<6} {:>4} {:>4} {:>4} {:>4} {:>4}  {:>9}  {:>5}(x{:.2})  {:>6}",
        "Op", "AR", "D1R", "D2R", "RR", "D1L", "out_level", "freq_r", 0.0_f32, "detune");
    println!("{}", "-".repeat(70));
    for (i, op) in v.ops.iter().enumerate() {
        let ratio = FREQ_RATIOS[(op.freq_ratio as usize).min(63)];
        let freq_hz = base_hz * ratio;
        let detune_cents = (op.detune as f32 - 3.0) * 3.0;
        println!("OP{}    {:>4} {:>4} {:>4} {:>4} {:>4}  {:>9}  {:>5}(x{:.2} ={:>7.1}Hz) {:>+.0}ct",
            i + 1, op.ar, op.d1r, op.d2r, op.rr, op.d1l,
            op.out_level, op.freq_ratio, ratio, freq_hz, detune_cents);
    }
    println!();
}
