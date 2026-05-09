/// Compare RMS envelope of HW recording vs SW render side-by-side.
/// Prints 20ms bins in bar-chart form.
///
/// Usage:
///   cargo run -p xdx-e2e --example compare_envelope -- \
///       <hw.wav> <bank.syx> <voice_idx>
use xdx_core::sysex::dx100_decode_32voice;
use xdx_synth::FmEngine;

const SR: f32 = 44100.0;
const MIDI_NOTE: u8 = 69;
const VELOCITY: u8 = 100;
const WINDOW_MS: f32 = 20.0;
const HOLD_MS: f32 = 3000.0;

fn rms_bins(samples: &[f32], sr: f32, win_ms: f32) -> Vec<f32> {
    let win = (sr * win_ms / 1000.0) as usize;
    let peak = samples.iter().cloned().fold(0.0_f32, f32::max);
    let norm: Vec<f32> = if peak > 0.0 {
        samples.iter().map(|&s| s / peak).collect()
    } else {
        samples.to_vec()
    };
    norm.chunks(win)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect()
}

fn read_wav(path: &str) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let hw_path = args
        .get(0)
        .map(|s| s.as_str())
        .unwrap_or("calibration/power_brass/dx100/01_PB_test.wav");
    let syx_path = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("calibration/power_brass/based_on_power_brass.syx");
    let voice_idx: usize = args
        .get(2)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1)
        .saturating_sub(1);

    // ── HW ───────────────────────────────────────────────────────────────────
    let hw_samples = read_wav(hw_path);
    let hw_sr = {
        let reader = hound::WavReader::open(hw_path).unwrap();
        reader.spec().sample_rate as f32
    };
    let hw_bins = rms_bins(&hw_samples, hw_sr, WINDOW_MS);

    // ── SW ───────────────────────────────────────────────────────────────────
    let bytes = std::fs::read(syx_path).unwrap_or_else(|e| panic!("read {syx_path}: {e}"));
    let voices = dx100_decode_32voice(&bytes).unwrap_or_else(|e| panic!("decode: {e:?}"));
    let voice = voices
        .get(voice_idx)
        .unwrap_or_else(|| panic!("voice out of range"));
    println!("SW voice {}: \"{}\"", voice_idx + 1, voice.name_str());

    let hold_samples = (HOLD_MS * SR / 1000.0) as usize;
    let mut engine = FmEngine::new(SR);
    engine.set_voice(voice.clone());
    engine.note_on(MIDI_NOTE, VELOCITY);
    let mut sw_raw = vec![0.0f32; hold_samples];
    let mut buf = vec![0.0f32; 512];
    let mut pos = 0;
    while pos < hold_samples {
        let chunk = buf.len().min(hold_samples - pos);
        buf[..chunk].fill(0.0);
        engine.render(&mut buf[..chunk]);
        sw_raw[pos..pos + chunk].copy_from_slice(&buf[..chunk]);
        pos += chunk;
    }
    let sw_bins = rms_bins(&sw_raw, SR, WINDOW_MS);

    // ── Normalise to peak=1.0 ────────────────────────────────────────────────
    let hw_peak = hw_bins.iter().cloned().fold(0.0_f32, f32::max);
    let sw_peak = sw_bins.iter().cloned().fold(0.0_f32, f32::max);
    let hw_norm: Vec<f32> = hw_bins.iter().map(|b| b / hw_peak.max(1e-9)).collect();
    let sw_norm: Vec<f32> = sw_bins.iter().map(|b| b / sw_peak.max(1e-9)).collect();

    // ── Print ────────────────────────────────────────────────────────────────
    println!("{:>7}  {:>6}  {:>6}  HW (H) vs SW (S)", "t(ms)", "HW", "SW");
    println!("{}", "-".repeat(60));
    let n = hw_norm.len().min(sw_norm.len()).min(160); // up to 3.2s
    for i in 0..n {
        let t_ms = i as f32 * WINDOW_MS + WINDOW_MS * 0.5;
        let hw = hw_norm[i];
        let sw = sw_norm.get(i).copied().unwrap_or(0.0);
        let hw_bar = "#".repeat((hw * 30.0).round() as usize);
        let sw_bar = ".".repeat((sw * 30.0).round() as usize);
        println!("{:>7.0}  {:>6.3}  {:>6.3}  H:{}", t_ms, hw, sw, hw_bar);
        println!("{:>7}  {:>6}  {:>6}  S:{}", "", "", "", sw_bar);
    }
}
