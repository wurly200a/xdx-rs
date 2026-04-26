//! Numerical analysis of lfo_calib recordings.
//! Measures lfo_speed→Hz, pitch-mod depth (cents), amp-mod depth (dB),
//! and lfo_delay onset from DX100 hardware recordings.
//!
//! Signal conditioning
//! -------------------
//!   The raw 16-bit WAV samples are first filtered through an 8-tap box lowpass
//!   (first null at 6 kHz) to remove ADC dither crossings, then amplitude-gated
//!   at -20 dB below the recording peak to suppress silence/noise crossings.
//!   All groups derive their frequency estimates from this cleaned ZCR series.
//!
//! Group A – lfo_speed:
//!   ZCR series interpolated to a 2 ms uniform time grid.
//!   LFO period estimated from the mean-crossing rate of the frequency deviation
//!   series (more robust than autocorrelation for asymmetric TRI waveforms).
//!   speed=0 (~0.063 Hz, period ~16 s) requires a >32 s recording to detect.
//!
//! Group B – pitch depth:
//!   ZCR series smoothed over 5 carrier cycles.
//!   5th/95th percentile frequencies converted to cents relative to the median.
//!
//! Group C – amp-mod depth:
//!   10 ms RMS bins; peak-to-trough ratio in dB over the middle 80% of the note.
//!
//! Group D – lfo_delay:
//!   ZCR series smoothed over 3 cycles; baseline from first 300 ms of note.
//!   Onset = first time where instantaneous pitch deviates > threshold_cents.
//!
//! Usage:
//!   cargo run -p xdx-compare --bin analyze-lfo-calib --release -- [--dir out/lfo_calib/grp_a]

use hound::WavReader;

// DEXED lfoSource table (Hz) — DX7 reference, index 0-99.
// Used as the expected values for Group A comparison.
const LFO_SOURCE: [f32; 100] = [
    0.062541, 0.125031, 0.312393, 0.437120, 0.624610, 0.750694, 0.936330, 1.125302, 1.249609,
    1.436782, 1.560915, 1.752081, 1.875117, 2.062494, 2.247191, 2.374451, 2.560492, 2.686728,
    2.873976, 2.998950, 3.188013, 3.369840, 3.500175, 3.682224, 3.812065, 4.000800, 4.186202,
    4.310716, 4.501260, 4.623209, 4.814636, 4.930480, 5.121901, 5.315191, 5.434783, 5.617346,
    5.750431, 5.946717, 6.062811, 6.248438, 6.431695, 6.564264, 6.749460, 6.868132, 7.052186,
    7.250580, 7.375719, 7.556294, 7.687577, 7.877738, 7.993605, 8.181967, 8.372405, 8.504848,
    8.685079, 8.810573, 8.986341, 9.122423, 9.300595, 9.500285, 9.607994, 9.798158, 9.950249,
    10.117361, 11.251125, 11.384335, 12.562814, 13.676149, 13.904338, 15.092062, 16.366612,
    16.638935, 17.869907, 19.193858, 19.425019, 20.833333, 21.034918, 22.502250, 24.003841,
    24.260068, 25.746653, 27.173913, 27.578599, 29.052876, 30.693677, 31.191516, 32.658393,
    34.317090, 34.674064, 36.416606, 38.197097, 38.550501, 40.387722, 40.749796, 42.625746,
    44.326241, 44.883303, 46.772685, 48.590865, 49.261084,
];

// ── Signal preprocessing ─────────────────────────────────────────────────────

/// Centered moving-average lowpass filter.
/// window=8 at 48 kHz gives first null at 6 kHz, passing the pitch-modulated carrier
/// (up to ~3 kHz for ±3 octave vibrato on C4) while eliminating ADC dither crossings.
fn boxcar_lowpass(samples: &[f32], window: usize) -> Vec<f32> {
    let n = samples.len();
    let half = window / 2;
    let mut cumsum = vec![0.0f32; n + 1];
    for i in 0..n {
        cumsum[i + 1] = cumsum[i] + samples[i];
    }
    (0..n)
        .map(|i| {
            let s = i.saturating_sub(half);
            let e = (i + half + 1).min(n);
            (cumsum[e] - cumsum[s]) / (e - s) as f32
        })
        .collect()
}

// ── WAV helpers ───────────────────────────────────────────────────────────────

fn load_wav_mono_f32(path: &str) -> Option<(Vec<f32>, f32)> {
    let mut reader = WavReader::open(path).ok()?;
    let spec = reader.spec();
    let sr = spec.sample_rate as f32;
    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i32>()
            .map(|s| s.unwrap() as f32 / (1i64 << (spec.bits_per_sample - 1)) as f32)
            .collect(),
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
    };
    let mono: Vec<f32> = if spec.channels == 2 {
        raw.chunks(2).map(|c| c[0]).collect()
    } else {
        raw
    };
    Some((mono, sr))
}

// ── Zero-crossing frequency series ───────────────────────────────────────────

/// Returns (time_s, inst_freq_hz) for each positive-slope zero crossing,
/// smoothed over `smooth` consecutive crossing intervals.
fn zcr_freq_series(samples: &[f32], sr: f32, smooth: usize) -> Vec<(f32, f32)> {
    // Lowpass before ZCR: null at ~6 kHz removes ADC dither while passing the
    // pitch-modulated carrier (up to ~3 kHz for ±3 oct vibrato on C4).
    let filtered = boxcar_lowpass(samples, 8);

    // Amplitude gate: skip crossings during silence/noise.
    // Compute 10 ms local energy and reject samples below -20 dB of the recording peak.
    let win = ((sr * 0.010) as usize).max(1);
    let sq: Vec<f32> = filtered.iter().map(|x| x * x).collect();
    let local_energy = boxcar_lowpass(&sq, win);
    let peak_energy = local_energy.iter().cloned().fold(0.0f32, f32::max);
    let gate = peak_energy * 0.01;

    // Positive-slope zero crossings within the active signal region
    let crossings: Vec<usize> = (1..filtered.len())
        .filter(|&i| filtered[i - 1] < 0.0 && filtered[i] >= 0.0 && local_energy[i] > gate)
        .collect();

    if crossings.len() < smooth + 2 {
        return Vec::new();
    }
    // Smooth by averaging `smooth` consecutive intervals
    crossings
        .windows(smooth + 1)
        .map(|w| {
            let period = (w[smooth] - w[0]) as f32 / smooth as f32;
            let t = w[smooth] as f32 / sr;
            (t, sr / period)
        })
        .collect()
}

// ── Group A: LFO frequency via time-domain autocorrelation ───────────────────

fn estimate_lfo_hz(samples: &[f32], sr: f32) -> Option<f32> {
    let raw = zcr_freq_series(samples, sr, 1);
    if raw.len() < 50 {
        return None;
    }
    // Clip to plausible carrier range to remove attack/release spikes
    let series: Vec<(f32, f32)> = raw
        .iter()
        .map(|&(t, f)| (t, f.clamp(20.0, 4000.0)))
        .collect();

    // Interpolate to a 2 ms uniform time grid.
    // Working in true time space avoids the non-uniform-sampling bias of ZCR-index space;
    // each grid step represents the same real duration regardless of carrier frequency.
    let dt = 0.002f32;
    let t0 = series[0].0;
    let t1 = series[series.len() - 1].0;
    if t1 - t0 < 1.0 {
        return None;
    }
    let n_grid = ((t1 - t0) / dt) as usize;
    let mut grid = vec![0.0f32; n_grid];
    let mut j = 0usize;
    for i in 0..n_grid {
        let t = t0 + i as f32 * dt;
        while j + 1 < series.len() - 1 && series[j + 1].0 <= t {
            j += 1;
        }
        let (t_a, f_a) = series[j];
        let (t_b, f_b) = series[(j + 1).min(series.len() - 1)];
        let alpha = if t_b > t_a {
            ((t - t_a) / (t_b - t_a)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        grid[i] = f_a + alpha * (f_b - f_a);
    }

    let mean = grid.iter().sum::<f32>() / n_grid as f32;
    let dev: Vec<f32> = grid.iter().map(|&f| f - mean).collect();
    let var = dev.iter().map(|x| x * x).sum::<f32>() / n_grid as f32;
    if var < 1.0 {
        return None; // < 1 Hz RMS deviation — no detectable LFO modulation
    }

    // Estimate LFO period from mean-crossing rate of the frequency deviation series.
    // The LFO modulates the carrier frequency as dev(t), which crosses zero twice per
    // LFO cycle. This is more robust than autocorrelation for asymmetric/distorted waveforms.
    // Apply a short smoothing pass first to suppress residual transient spikes.
    let smooth_n = 10usize; // 20 ms smoothing window
    let dev_sm: Vec<f32> = (0..n_grid)
        .map(|i| {
            let s = i.saturating_sub(smooth_n);
            let e = (i + smooth_n + 1).min(n_grid);
            dev[s..e].iter().sum::<f32>() / (e - s) as f32
        })
        .collect();

    let crossings: Vec<usize> = (1..dev_sm.len())
        .filter(|&i| {
            (dev_sm[i - 1] < 0.0 && dev_sm[i] >= 0.0) || (dev_sm[i - 1] >= 0.0 && dev_sm[i] < 0.0)
        })
        .collect();

    if crossings.len() < 4 {
        return None;
    }
    let n_cross = crossings.len();
    let span = (crossings[n_cross - 1] - crossings[0]) as f32 * dt;
    let lfo_period = 2.0 * span / (n_cross - 1) as f32;
    if lfo_period < 0.01 || lfo_period > 20.0 {
        return None;
    }
    Some(1.0 / lfo_period)
}

// ── Group B: pitch deviation in cents ────────────────────────────────────────

fn estimate_pitch_cents(samples: &[f32], sr: f32) -> (f32, f32) {
    let series = zcr_freq_series(samples, sr, 5); // 5-cycle smoothing
    if series.is_empty() {
        return (0.0, 0.0);
    }
    // Discard first and last 0.5 s
    let t_start = series[0].0 + 0.5;
    let t_end = series[series.len() - 1].0 - 0.5;
    let steady: Vec<f32> = series
        .iter()
        .filter(|&&(t, _)| t >= t_start && t <= t_end)
        .map(|&(_, f)| f)
        .collect();
    if steady.is_empty() {
        return (0.0, 0.0);
    }
    // Use percentile-based min/max to reject outlier ZCR values
    let mut sorted = steady.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pct = |p: f32| sorted[((sorted.len() as f32 - 1.0) * p) as usize];
    let mean = pct(0.50); // median as reference (robust against drift)
    let max_f = pct(0.95);
    let min_f = pct(0.05);
    let up = 1200.0 * (max_f / mean).log2();
    let dn = 1200.0 * (mean / min_f).log2(); // magnitude of downward deviation
    (up, dn)
}

// ── Group C: amplitude-mod depth in dB ───────────────────────────────────────

fn rms_bins(samples: &[f32], sr: f32, win_ms: f32) -> Vec<f32> {
    let win = (sr * win_ms / 1000.0) as usize;
    samples
        .chunks(win)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect()
}

fn estimate_amp_mod_db(samples: &[f32], sr: f32) -> f32 {
    let bins = rms_bins(samples, sr, 10.0);
    let total = bins.len();
    // Use middle 80% to skip attack and release
    let s = total / 10;
    let e = total * 9 / 10;
    let steady = &bins[s..e];
    if steady.is_empty() {
        return 0.0;
    }
    let peak = steady.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let trough = steady
        .iter()
        .cloned()
        .fold(f32::INFINITY, f32::min)
        .max(1e-9);
    20.0 * (peak / trough).log10()
}

// ── Group D: LFO delay onset ──────────────────────────────────────────────────

/// Returns onset time (ms) from first ZCR where LFO modulation exceeds `threshold_cents`.
fn estimate_delay_onset_ms(samples: &[f32], sr: f32, threshold_cents: f32) -> Option<f32> {
    let series = zcr_freq_series(samples, sr, 3);
    if series.is_empty() {
        return None;
    }
    // Use first 300 ms of the series (relative to first valid ZCR) as baseline.
    // Absolute t=0 is unreliable because the recording may have pre-roll silence.
    let t_note_start = series[0].0;
    let t_baseline_end = t_note_start + 0.3;
    let baseline: Vec<f32> = series
        .iter()
        .filter(|&&(t, _)| t < t_baseline_end)
        .map(|&(_, f)| f)
        .collect();
    if baseline.is_empty() {
        return None;
    }
    let mean_f = baseline.iter().sum::<f32>() / baseline.len() as f32;

    for &(t, f) in &series {
        if t < t_baseline_end {
            continue;
        }
        let cents = (1200.0 * (f / mean_f).log2()).abs();
        if cents > threshold_cents {
            return Some((t - t_note_start) * 1000.0);
        }
    }
    None
}

// ── main ─────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args
        .windows(2)
        .find(|w| w[0] == "--dir")
        .map(|w| w[1].as_str())
        .unwrap_or("out/lfo_calib/grp_a");
    let dx100 = format!("{dir}/dx100");

    println!("=== LFO calibration analysis  ({dx100}) ===\n");

    // ── Group A: lfo_speed → Hz ───────────────────────────────────────────────
    let speed_voices: &[(u8, &str)] = &[
        (0, "01_SPD_00.wav"),
        (16, "02_SPD_16.wav"),
        (33, "03_SPD_33.wav"),
        (50, "04_SPD_50.wav"),
        (66, "05_SPD_66.wav"),
        (83, "06_SPD_83.wav"),
        (99, "07_SPD_99.wav"),
    ];
    println!("── Group A: lfo_speed → Hz ──────────────────────────────────────────");
    println!(
        "  {:>5}  {:>10}  {:>10}  {:>8}",
        "speed", "measured", "DX7-ref", "diff%"
    );
    for &(speed, file) in speed_voices {
        let path = format!("{dx100}/{file}");
        let exp = LFO_SOURCE[speed as usize];
        if let Some((samples, sr)) = load_wav_mono_f32(&path) {
            match estimate_lfo_hz(&samples, sr) {
                Some(meas) => {
                    let diff = (meas - exp) / exp * 100.0;
                    println!(
                        "  {:>5}  {:>10.3} Hz  {:>10.3} Hz  {:>+7.1}%",
                        speed, meas, exp, diff
                    );
                }
                None => println!(
                    "  {:>5}  {:>10}  {:>10.3} Hz  (cannot estimate — too slow/fast or no mod)",
                    speed, "—", exp
                ),
            }
        } else {
            println!("  {:>5}  (file not found: {path})", speed);
        }
    }

    // ── Group B: pitch-mod depth ──────────────────────────────────────────────
    let pmd_voices: &[(u8, u8, &str)] = &[
        (50, 3, "08_PMD50_S3.wav"),
        (99, 3, "09_PMD99_S3.wav"),
        (50, 7, "10_PMD50_S7.wav"),
        (99, 7, "11_PMD99_S7.wav"),
    ];
    println!("\n── Group B: pitch-mod depth ─────────────────────────────────────────");
    println!(
        "  {:>5}  {:>5}  {:>10}  {:>10}",
        "PMD", "PMS", "+cents", "-cents"
    );
    for &(pmd, pms, file) in pmd_voices {
        let path = format!("{dx100}/{file}");
        if let Some((samples, sr)) = load_wav_mono_f32(&path) {
            let (up, dn) = estimate_pitch_cents(&samples, sr);
            println!("  {:>5}  {:>5}  {:>+9.1}¢  {:>+9.1}¢", pmd, pms, up, -dn);
        } else {
            println!("  {:>5}  {:>5}  (file not found: {path})", pmd, pms);
        }
    }

    // ── Group C: amp-mod depth ────────────────────────────────────────────────
    let amd_voices: &[(u8, &str)] = &[
        (1, "12_AMD99_A1.wav"),
        (2, "13_AMD99_A2.wav"),
        (3, "14_AMD99_A3.wav"),
    ];
    println!("\n── Group C: amp-mod depth (AMD=99) ──────────────────────────────────");
    println!("  {:>5}  {:>12}", "AMS", "peak-trough dB");
    for &(ams, file) in amd_voices {
        let path = format!("{dx100}/{file}");
        if let Some((samples, sr)) = load_wav_mono_f32(&path) {
            let db = estimate_amp_mod_db(&samples, sr);
            println!("  {:>5}  {:>11.1} dB", ams, db);
        } else {
            println!("  {:>5}  (file not found: {path})", ams);
        }
    }

    // ── Group D: lfo_delay onset ─────────────────────────────────────────────
    let delay_voices: &[(u8, &str)] = &[
        (25, "15_DLY_025.wav"),
        (50, "16_DLY_050.wav"),
        (75, "17_DLY_075.wav"),
    ];
    println!("\n── Group D: lfo_delay onset ─────────────────────────────────────────");
    println!("  {:>8}  {:>15}", "delay", "onset (ms)");
    for &(delay, file) in delay_voices {
        let path = format!("{dx100}/{file}");
        if let Some((samples, sr)) = load_wav_mono_f32(&path) {
            match estimate_delay_onset_ms(&samples, sr, 10.0) {
                Some(ms) => println!("  {:>8}  {:>14.0} ms", delay, ms),
                None => println!("  {:>8}  (no onset detected)", delay),
            }
        } else {
            println!("  {:>8}  (file not found: {path})", delay);
        }
    }

    println!();
}
