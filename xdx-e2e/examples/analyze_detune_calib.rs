/// Measure beat frequency from detune calibration recordings.
/// Reads WAV files from <dir>/dx100/, computes the beat frequency via envelope
/// autocorrelation, then derives the per-step detune magnitude in Hz and cents.
///
/// Usage:
///   cargo run -p xdx-e2e --example analyze_detune_calib -- testdata/wav/detune_calib
///
/// Output table columns:
///   Beat Hz    – measured beat between the two detuned carriers
///   Hz/step    – absolute frequency offset per detune step
///   Cents/step – relative pitch offset per detune step
///
/// Voices recorded at A3 (220 Hz) vs A4 (440 Hz):
///   If Hz/step stays constant   → detune is Hz-based (absolute offset)
///   If Cents/step stays constant → detune is cents-based (proportional)
use std::path::PathBuf;

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: analyze_detune_calib <dir>");
        std::process::exit(1);
    });

    let dx100_dir = PathBuf::from(&dir).join("dx100");
    if !dx100_dir.exists() {
        eprintln!("No dx100/ subdirectory found in {dir}");
        std::process::exit(1);
    }

    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dx100_dir)
        .expect("read dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "wav").unwrap_or(false))
        .collect();
    entries.sort();

    println!(
        "{:<20} {:>5} {:>7} {:>10} {:>10} {:>12}",
        "Voice", "Steps", "Base Hz", "Beat Hz", "Hz/step", "Cents/step"
    );
    println!("{}", "-".repeat(68));

    for path in &entries {
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let (steps, carrier_hz) = parse_voice_name(&stem);

        let (samples, sr) = load_wav(&path.to_string_lossy());
        let duration_s = samples.len() as f32 / sr;
        let beat_hz = measure_beat_freq(&samples, sr, 10.0);

        // Require at least 1.5 beat cycles within the recording; reject otherwise.
        let min_detectable = 1.5 / duration_s;
        let (beat_str, hz_str, cents_str) =
            if steps != 0 && carrier_hz > 0.0 && beat_hz > 0.0 && beat_hz >= min_detectable {
                let n = steps.abs() as f32;
                let hz = beat_hz / n;
                let cents = 1200.0 / n * (1.0 + beat_hz / carrier_hz).log2();
                (
                    format!("{:10.4}", beat_hz),
                    format!("{:10.4}", hz),
                    format!("{:12.4}", cents),
                )
            } else {
                (
                    "   (too slow)".into(),
                    "          -".into(),
                    "           -".into(),
                )
            };

        println!(
            "{:<20} {:>5} {:>7.0} {} {} {}",
            stem.chars().take(20).collect::<String>(),
            steps,
            carrier_hz,
            beat_str,
            hz_str,
            cents_str,
        );
    }

    println!();
    println!("Interpretation:");
    println!("  Hz/step constant across 220Hz/440Hz rows → detune is Hz-based");
    println!("  Cents/step constant across rows           → detune is cents-based");
    println!("  Current xdx-synth uses: detune_cents = (raw - 3) * COEFF");
    println!("  Set COEFF to the Cents/step value from the 440Hz rows.");
}

/// Parse step count and carrier frequency from a filename like "03_+3_440Hz".
fn parse_voice_name(stem: &str) -> (i32, f32) {
    let after_nn = stem.splitn(2, '_').nth(1).unwrap_or("");
    let parts: Vec<&str> = after_nn.split('_').collect();

    let steps = parts
        .first()
        .and_then(|s| {
            if s.starts_with('+') {
                s[1..].parse::<i32>().ok()
            } else if s.starts_with('-') {
                s[1..].parse::<i32>().ok().map(|n| -n)
            } else {
                None
            }
        })
        .unwrap_or(0);

    let carrier_hz = parts
        .get(1)
        .and_then(|s| s.strip_suffix("Hz"))
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.0);

    (steps, carrier_hz)
}

fn load_wav(path: &str) -> (Vec<f32>, f32) {
    let mut reader = hound::WavReader::open(path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    let sr = reader.spec().sample_rate as f32;
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();
    (samples, sr)
}

/// Measure beat frequency via normalized RMS-envelope autocorrelation.
/// Returns 0.0 when no significant periodic pattern is found.
/// `window_ms`: RMS window; should be >> audio cycle period, << beat period.
fn measure_beat_freq(samples: &[f32], sr: f32, window_ms: f32) -> f32 {
    let window = ((window_ms / 1000.0) * sr) as usize;
    if window == 0 || samples.len() < window * 8 {
        return 0.0;
    }

    // RMS envelope
    let envelope: Vec<f32> = samples
        .chunks(window)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect();

    let n = envelope.len();
    let mean = envelope.iter().sum::<f32>() / n as f32;
    let centered: Vec<f32> = envelope.iter().map(|s| s - mean).collect();

    // Variance for normalization
    let variance = centered.iter().map(|s| s * s).sum::<f32>() / n as f32;
    if variance < 1e-12 {
        return 0.0;
    }

    let env_sr = sr / window as f32;
    let min_lag = (0.3 * env_sr) as usize; // minimum 0.3s period
    let max_lag = ((40.0 * env_sr) as usize).min(n / 2);
    if max_lag <= min_lag {
        return 0.0;
    }

    // Normalized autocorrelation: 1.0 at lag=0, range roughly [-1, 1]
    let autocorr: Vec<f32> = (min_lag..max_lag)
        .map(|lag| {
            let len = n - lag;
            centered[..len]
                .iter()
                .zip(&centered[lag..])
                .map(|(a, b)| a * b)
                .sum::<f32>()
                / (len as f32 * variance)
        })
        .collect();

    // Require a local peak with normalized correlation > 0.15 to reject noise/artifact.
    // A genuine beat produces a clear peak; non-stationary EG shapes give much weaker correlation.
    let peak = autocorr
        .iter()
        .enumerate()
        .filter(|&(i, &v)| {
            let prev = if i > 0 {
                autocorr[i - 1]
            } else {
                f32::NEG_INFINITY
            };
            let next = autocorr.get(i + 1).copied().unwrap_or(f32::NEG_INFINITY);
            v > prev && v > next && v > 0.15
        })
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());

    if let Some((i, _)) = peak {
        let period_s = (i + min_lag) as f32 / env_sr;
        1.0 / period_s
    } else {
        0.0
    }
}
