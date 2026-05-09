/// Scan initial fb_prev / fb_prev2 state to check whether non-zero feedback
/// carry-over from a previous note can reproduce the HW PowerBrass undulation.
///
/// In the DX100 hardware, the OP4 feedback register may retain its value
/// from the end of the previous voice (Horns, voice 10) when the next note
/// starts.  xdx-synth resets fb_prev=0 / fb_prev2=0 at every note_on.
///
/// This scan tries fb_prev in [-1.0, 1.0] (step 0.1) while fb_prev2=fb_prev/2,
/// and reports the beat depth at t≈740ms (trough expected in HW).
///
/// Usage:
///   cargo run -p xdx-e2e --example scan_fb_state -- \
///       calibration/preset_bank_wo_lfo/all_voices_wo_lfo.syx 11
use xdx_core::sysex::dx100_decode_32voice;
use xdx_synth::FmEngine;

const SR: f32 = 44100.0;
const MIDI_NOTE: u8 = 69;
const VELOCITY: u8 = 100;
const WINDOW_MS: f32 = 10.0;
const HOLD_MS: f32 = 1200.0;

fn rms_bin(samples: &[f32], t_ms: f32) -> f32 {
    let win = (SR * WINDOW_MS / 1000.0) as usize;
    let idx = (t_ms / WINDOW_MS) as usize;
    let start = idx * win;
    let end = (start + win).min(samples.len());
    if end <= start {
        return 0.0;
    }
    let s = &samples[start..end];
    (s.iter().map(|v| v * v).sum::<f32>() / s.len() as f32).sqrt()
}

fn render_with_fb(engine: &mut FmEngine, midi_note: u8, fb_prev: f32, fb_prev2: f32) -> Vec<f32> {
    let hold_samples = (HOLD_MS * SR / 1000.0) as usize;
    engine.note_on_with_fb(midi_note, VELOCITY, fb_prev, fb_prev2);

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
    engine.note_off(midi_note);
    samples
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let syx_path = args.get(0).map(|s| s.as_str()).unwrap_or(
        "calibration/preset_bank_wo_lfo/all_voices_wo_lfo.syx",
    );
    let voice_idx: usize = args
        .get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(11)
        .saturating_sub(1);

    let bytes = std::fs::read(syx_path).unwrap_or_else(|e| panic!("read {syx_path}: {e}"));
    let voices = dx100_decode_32voice(&bytes).unwrap_or_else(|e| panic!("decode: {e:?}"));
    let voice = voices.get(voice_idx).unwrap_or_else(|| panic!("out of range"));

    println!("Voice {}: \"{}\"  (FB={})", voice_idx + 1, voice.name_str(), voice.feedback);
    println!("HW ref: rms@30ms≈0.955  rms@740ms≈0.542  rms@1060ms≈1.000");
    println!();
    println!(
        "{:>8}  {:>9}  {:>9}  {:>10}  {:>10}",
        "fb_prev", "rms@30ms", "rms@740ms", "rms@1060ms", "beat_depth"
    );
    println!("{}", "-".repeat(55));

    let steps = 20;
    for step in 0..=steps {
        let fb_prev = -1.0 + 2.0 * step as f32 / steps as f32;
        let fb_prev2 = fb_prev * 0.5;

        let mut engine = FmEngine::new(SR);
        engine.set_voice(voice.clone());

        let samples = render_with_fb(&mut engine, MIDI_NOTE, fb_prev, fb_prev2);

        let peak = samples.iter().cloned().fold(0.0_f32, f32::max);
        let norm: Vec<f32> = if peak > 0.0 {
            samples.iter().map(|&s| s / peak).collect()
        } else {
            samples
        };

        let r30 = rms_bin(&norm, 30.0);
        let r740 = rms_bin(&norm, 740.0);
        let r1060 = rms_bin(&norm, 1060.0);
        let beat_depth = (r30 - r740).max(r1060 - r740);
        let mark = if beat_depth > 0.2 { " <<<" } else { "" };

        println!(
            "{:>8.3}  {:>9.4}  {:>9.4}  {:>10.4}  {:>10.4}{}",
            fb_prev, r30, r740, r1060, beat_depth, mark
        );
    }
}
