/// Scan OP4 initial phase to find the value that reproduces the HW amplitude
/// undulation (~1030ms period) observed in the DX100 PowerBrass recording.
///
/// Theory: DX100 does not reset phase accumulators on key-on.  OP4 at ratio=3
/// (660 Hz) combined with OP1-3 at ratio=1+det=0 (219.679 Hz) creates a beat
/// at |660 - 3×219.679| ≈ 0.964 Hz (period ≈ 1038ms).  xdx-synth initialises
/// all phases to 0.0 which suppresses the beat; a different OP4 phase can
/// excite it.
///
/// For each candidate OP4 phase the example renders 1200ms and reports the
/// normalised RMS at three key time-points that characterise the HW pattern:
///   t≈30ms   (first peak after attack)
///   t≈740ms  (first trough)
///   t≈1060ms (second peak / global max)
///
/// HW reference (from compare_eg):
///   rms_30   ≈ 0.955   rms_740 ≈ 0.542   rms_1060 ≈ 1.000
///
/// Usage:
///   cargo run -p xdx-e2e --example scan_op4_phase -- <bank.syx> <voice_index>
///   cargo run -p xdx-e2e --example scan_op4_phase -- \
///       calibration/preset_bank_wo_lfo/all_voices_wo_lfo.syx 11
use xdx_core::sysex::dx100_decode_32voice;
use xdx_synth::FmEngine;

const SR: f32 = 44100.0;
const MIDI_NOTE: u8 = 69;
const VELOCITY: u8 = 100;
const WINDOW_MS: f32 = 10.0;
const HOLD_MS: f32 = 1200.0;

fn rms_at_bin(bins: &[f32], t_ms: f32) -> f32 {
    let idx = (t_ms / WINDOW_MS) as usize;
    bins.get(idx).copied().unwrap_or(0.0)
}

fn render_bins(engine: &mut FmEngine, midi_note: u8) -> Vec<f32> {
    let hold_samples = (HOLD_MS * SR / 1000.0) as usize;
    let win = (SR * WINDOW_MS / 1000.0) as usize;

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

    // normalise
    let peak = samples.iter().cloned().fold(0.0_f32, f32::max);
    if peak > 0.0 {
        for s in samples.iter_mut() {
            *s /= peak;
        }
    }

    samples
        .chunks(win)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect()
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
    let voices =
        dx100_decode_32voice(&bytes).unwrap_or_else(|e| panic!("decode failed: {e:?}"));
    let voice = voices
        .get(voice_idx)
        .unwrap_or_else(|| panic!("voice index {} out of range", voice_idx + 1));

    println!("Voice {}: \"{}\"", voice_idx + 1, voice.name_str());
    println!("Beat period = 1/|f_OP4 - 3×f_OP1| ≈ 1038ms");
    println!();
    println!(
        "HW reference:  rms@30ms≈0.955  rms@740ms≈0.542  rms@1060ms≈1.000  beat_depth≈0.84"
    );
    println!();
    println!(
        "{:>8}  {:>9}  {:>9}  {:>10}  {:>10}  note",
        "op4_ph", "rms@30ms", "rms@740ms", "rms@1060ms", "beat_depth"
    );
    println!("{}", "-".repeat(60));

    // Scan OP4 phase in 0.05 steps; all other operators stay at 0.
    let steps = 20;
    let mut best_depth = -1.0f32;
    let mut best_phase = 0.0f32;

    for step in 0..=steps {
        let op4_phase = step as f64 / steps as f64;
        let op_phases = [0.0_f64, 0.0, 0.0, op4_phase];

        let mut engine = FmEngine::new(SR);
        engine.set_voice(voice.clone());
        engine.note_on_with_phases(MIDI_NOTE, VELOCITY, op_phases);

        let bins = render_bins(&mut engine, MIDI_NOTE);

        // Normalise bins to peak=1 (same as compare_eg does)
        let peak = bins.iter().cloned().fold(0.0_f32, f32::max);
        let norm_bins: Vec<f32> = if peak > 0.0 {
            bins.iter().map(|&v| v / peak).collect()
        } else {
            bins
        };

        let r30 = rms_at_bin(&norm_bins, 30.0);
        let r740 = rms_at_bin(&norm_bins, 740.0);
        let r1060 = rms_at_bin(&norm_bins, 1060.0);
        // beat_depth: how much the envelope dips (1.0 = HW-like swing, 0.0 = flat)
        let beat_depth = (r30 - r740).max(r1060 - r740);

        let mark = if beat_depth > 0.3 { " <<<" } else { "" };
        println!(
            "{:>8.3}  {:>9.4}  {:>9.4}  {:>10.4}  {:>10.4}{}",
            op4_phase, r30, r740, r1060, beat_depth, mark
        );

        if beat_depth > best_depth {
            best_depth = beat_depth;
            best_phase = op4_phase as f32;
        }
    }

    println!();
    println!(
        "Best OP4 phase: {best_phase:.3}  beat_depth: {best_depth:.4}"
    );
}
