/// Render a DX100 1-voice SysEx file to a WAV using xdx-synth.
/// Usage:
///   cargo run -p xdx-e2e --example render_voice -- <syx_file> <midi_note> <hold_ms> <out.wav>
use hound::{SampleFormat, WavSpec, WavWriter};
use xdx_core::sysex::dx100_decode_1voice;
use xdx_synth::FmEngine;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        eprintln!("Usage: render_voice <syx> <midi_note> <hold_ms> <out.wav>");
        std::process::exit(1);
    }
    let syx_path = &args[0];
    let midi_note: u8 = args[1].parse().expect("midi_note");
    let hold_ms: u32 = args[2].parse().expect("hold_ms");
    let out_path = &args[3];

    const SR: f32 = 44100.0;
    const VELOCITY: u8 = 100;
    const RELEASE_MS: u32 = 500;

    let bytes = std::fs::read(syx_path).expect("read syx");
    let voice = dx100_decode_1voice(&bytes).expect("decode syx");

    let mut engine = FmEngine::new(SR);
    engine.set_voice(voice);
    engine.note_on(midi_note, VELOCITY);

    let hold_samples = (hold_ms as f32 * SR / 1000.0) as usize;
    let release_samples = (RELEASE_MS as f32 * SR / 1000.0) as usize;
    let total = hold_samples + release_samples;

    let spec = WavSpec {
        channels: 1,
        sample_rate: SR as u32,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(out_path, spec).expect("create wav");

    let mut buf = vec![0.0f32; 512];
    let mut written = 0usize;
    let mut released = false;

    while written < total {
        let chunk = buf.len().min(total - written);
        let buf_slice = &mut buf[..chunk];
        buf_slice.fill(0.0);

        if !released && written >= hold_samples {
            engine.note_off(midi_note);
            released = true;
        }

        engine.render(buf_slice);

        for &s in buf_slice.iter() {
            let clamped = s.clamp(-1.0, 1.0);
            writer
                .write_sample((clamped * i16::MAX as f32) as i16)
                .expect("write");
        }
        written += chunk;
    }

    writer.finalize().expect("finalize");
    println!(
        "Wrote {out_path}  ({total} samples, {}ms)",
        total as f32 / SR * 1000.0
    );
}
