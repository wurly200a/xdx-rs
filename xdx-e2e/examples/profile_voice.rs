/// Render a voice and print the RMS envelope profile in 10ms bins.
/// Useful for visually inspecting amplitude beats vs hardware recordings.
///
/// Usage:
///   cargo run -p xdx-e2e --example profile_voice -- <bank.syx> <voice_idx>
use xdx_core::sysex::dx100_decode_32voice;
use xdx_synth::FmEngine;

const SR: f32 = 44100.0;
const MIDI_NOTE: u8 = 69;
const VELOCITY: u8 = 100;
const WINDOW_MS: f32 = 10.0;
const HOLD_MS: f32 = 3000.0;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let syx_path = args.get(0).map(|s| s.as_str()).unwrap_or(
        "calibration/power_brass/based_on_power_brass.syx",
    );
    let voice_idx: usize = args
        .get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1)
        .saturating_sub(1);

    let bytes = std::fs::read(syx_path).unwrap_or_else(|e| panic!("read {syx_path}: {e}"));
    let voices = dx100_decode_32voice(&bytes).unwrap_or_else(|e| panic!("decode: {e:?}"));
    let voice = voices.get(voice_idx).unwrap_or_else(|| panic!("voice out of range"));

    println!("Voice {}: \"{}\"", voice_idx + 1, voice.name_str());
    println!();

    let hold_samples = (HOLD_MS * SR / 1000.0) as usize;
    let win = (SR * WINDOW_MS / 1000.0) as usize;

    let mut engine = FmEngine::new(SR);
    engine.set_voice(voice.clone());
    engine.note_on(MIDI_NOTE, VELOCITY);

    let mut samples = vec![0.0f32; hold_samples];
    let mut buf = vec![0.0f32; 512];
    let mut pos = 0;
    while pos < hold_samples {
        let chunk = buf.len().min(hold_samples - pos);
        buf[..chunk].fill(0.0);
        engine.render(&mut buf[..chunk]);
        samples[pos..pos + chunk].copy_from_slice(&buf[..chunk]);
        pos += chunk;
    }

    let peak = samples.iter().cloned().fold(0.0_f32, f32::max);
    if peak > 0.0 {
        for s in samples.iter_mut() {
            *s /= peak;
        }
    }

    let bins: Vec<f32> = samples
        .chunks(win)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect();

    let bin_peak = bins.iter().cloned().fold(0.0_f32, f32::max);
    let norm_bins: Vec<f32> = if bin_peak > 0.0 {
        bins.iter().map(|b| b / bin_peak).collect()
    } else {
        bins.clone()
    };

    println!("{:>6}  {:>7}  bar", "t(ms)", "rms");
    println!("{}", "-".repeat(50));
    for (i, &rms) in norm_bins.iter().enumerate() {
        let t_ms = i as f32 * WINDOW_MS + WINDOW_MS * 0.5;
        let bar_len = (rms * 40.0).round() as usize;
        let bar = "#".repeat(bar_len);
        println!("{:>6.0}  {:>7.4}  {}", t_ms, rms, bar);
    }
}
