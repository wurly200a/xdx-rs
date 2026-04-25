/// Compare attack onset characteristics between WAV files.
/// Measures time from sample 0 to when RMS first reaches 1%, 10%, 50% of peak.
///
/// Usage:
///   cargo run -p xdx-e2e --example compare_attack -- file1.wav [file2.wav ...]
use std::env;

fn analyse(path: &str) -> Option<()> {
    let mut reader = hound::WavReader::open(path).ok()?;
    let sr = reader.spec().sample_rate as f32;
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();

    // 1ms window RMS
    let win = (sr / 1000.0) as usize;
    let rmss: Vec<f32> = samples
        .chunks(win)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect();

    let peak_rms = rmss.iter().cloned().fold(0.0_f32, f32::max);
    if peak_rms == 0.0 {
        println!("{path}: silent");
        return Some(());
    }

    // Detect note onset: first window that exceeds 0.5% of peak
    // (accounts for pre-delay in hardware recordings)
    let noise_floor = peak_rms * 0.005;
    let onset_ms = rmss.iter().position(|&r| r > noise_floor).unwrap_or(0) as f32;

    let threshold = |frac: f32| -> Option<f32> {
        let thr = peak_rms * frac;
        rmss.iter()
            .position(|&r| r >= thr)
            .map(|i| i as f32 - onset_ms) // relative to onset
    };

    let t1 = threshold(0.01).unwrap_or(f32::NAN);
    let t10 = threshold(0.10).unwrap_or(f32::NAN);
    let t50 = threshold(0.50).unwrap_or(f32::NAN);
    let t90 = threshold(0.90).unwrap_or(f32::NAN);

    println!("{path}");
    println!(
        "  peak_rms={:.4}  onset_at={:.0}ms  rel: (1%)={:.1}ms  (10%)={:.1}ms  (50%)={:.1}ms  (90%)={:.1}ms",
        peak_rms, onset_ms, t1, t10, t50, t90
    );

    // Print 1ms RMS for 100ms around onset
    let start = onset_ms as usize;
    println!("  t_ms   rms");
    for (i, &rms) in rmss.iter().enumerate().skip(start).take(100) {
        let bar_len = (rms / peak_rms * 40.0) as usize;
        let bar = "#".repeat(bar_len);
        println!("  {:>+5}  {:.4}  {}", i as f32 - onset_ms, rms, bar);
    }
    println!();
    Some(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: compare_attack file1.wav [file2.wav ...]");
        return;
    }
    for path in &args {
        analyse(path);
    }
}
