/// Compare full EG envelopes between DX100 hardware recordings and softsynth renders.
///
/// Usage (batch – processes all matching pairs):
///   compare_eg --dir <dir> [--hold-ms <ms>]
///
/// Usage (single pair with full envelope table):
///   compare_eg <dx100.wav> <synth.wav> [--hold-ms <ms>]
///
/// --hold-ms : note hold duration in ms used during recording (default: 3000)
///
/// Metrics:
///   atk(90%)  – ms from onset to when normalized RMS first reaches 0.90
///   d1l       – mean normalized RMS during last 10% of hold window (sustain level)
///   rls(50%)  – ms from note-off for RMS to drop to 50% of note-off level
///   rls(90%)  – ms from note-off for RMS to drop to 10% of note-off level
use hound::WavReader;
use std::env;

const WINDOW_MS: f32 = 10.0; // RMS bin size

// ── WAV loading ───────────────────────────────────────────────────────────────

fn load_rms_bins(path: &str) -> Option<Vec<f32>> {
    let mut reader = WavReader::open(path).ok()?;
    let sr = reader.spec().sample_rate as f32;
    let win = (sr * WINDOW_MS / 1000.0) as usize;
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();
    let bins = samples
        .chunks(win)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect();
    Some(bins)
}

fn find_onset(bins: &[f32]) -> usize {
    let peak = bins.iter().cloned().fold(0.0_f32, f32::max);
    let thr = peak * 0.005;
    bins.iter().position(|&r| r > thr).unwrap_or(0)
}

// ── Metrics ───────────────────────────────────────────────────────────────────

struct EgMetrics {
    atk90_ms: f32, // ms from onset to normalized RMS ≥ 0.90
    d1l: f32,      // mean normalized RMS in last 10% of hold window
    rls50_ms: f32, // ms from note-off to 50% of note-off level
    rls90_ms: f32, // ms from note-off to 10% of note-off level
}

fn compute_metrics(bins: &[f32], onset: usize, hold_bins: usize) -> EgMetrics {
    let peak = bins.iter().cloned().fold(0.0_f32, f32::max);
    if peak < 1e-7 {
        return EgMetrics {
            atk90_ms: f32::NAN,
            d1l: 0.0,
            rls50_ms: f32::NAN,
            rls90_ms: f32::NAN,
        };
    }

    let get = |n: usize| bins.get(onset + n).copied().unwrap_or(0.0) / peak;

    // Attack 90%
    let atk90_ms = (0..hold_bins)
        .find(|&n| get(n) >= 0.9)
        .map(|n| n as f32 * WINDOW_MS)
        .unwrap_or(f32::NAN);

    // D1L: mean of last 10% of hold
    let d1l_start = hold_bins * 9 / 10;
    let d1l_count = hold_bins.saturating_sub(d1l_start).max(1);
    let d1l = (0..d1l_count).map(|i| get(d1l_start + i)).sum::<f32>() / d1l_count as f32;

    // Release from note-off
    let at_off = get(hold_bins);
    let rls_ms = |frac: f32| -> f32 {
        let thr = at_off * frac;
        (0..)
            .find(|&n| get(hold_bins + n) <= thr)
            .map(|n| n as f32 * WINDOW_MS)
            .unwrap_or(f32::NAN)
    };
    let rls50_ms = rls_ms(0.5);
    let rls90_ms = rls_ms(0.1);

    EgMetrics {
        atk90_ms,
        d1l,
        rls50_ms,
        rls90_ms,
    }
}

// ── Envelope table ────────────────────────────────────────────────────────────

fn print_envelope(
    dx_bins: &[f32],
    sy_bins: &[f32],
    dx_onset: usize,
    sy_onset: usize,
    hold_bins: usize,
) {
    let dx_peak = dx_bins.iter().cloned().fold(0.0_f32, f32::max);
    let sy_peak = sy_bins.iter().cloned().fold(0.0_f32, f32::max);
    let dx_get = |n: usize| dx_bins.get(dx_onset + n).copied().unwrap_or(0.0) / dx_peak;
    let sy_get = |n: usize| sy_bins.get(sy_onset + n).copied().unwrap_or(0.0) / sy_peak;

    let total =
        (dx_bins.len().saturating_sub(dx_onset)).min(sy_bins.len().saturating_sub(sy_onset));

    println!(
        "\n  {:>6}  {:>6}  {:>6}  {:>5}  {}",
        "t(ms)", "HW", "SY", "HW/SY", "bar (H=HW, S=SY)"
    );
    println!("  {}", "-".repeat(72));

    let mut prev_printed = false;
    for n in 0..total {
        let dv = dx_get(n);
        let sv = sy_get(n);
        let t = n as f32 * WINDOW_MS;
        let note_off = n == hold_bins;

        // Print denser during attack/release; skip flat sustain rows
        let in_attack = n < (200.0 / WINDOW_MS) as usize;
        let in_release = n >= hold_bins;
        let changed = (dv - dx_get(n.saturating_sub(1))).abs() > 0.005
            || (sv - sy_get(n.saturating_sub(1))).abs() > 0.005;
        let should_print = in_attack || in_release || changed || note_off || !prev_printed;

        if note_off {
            println!(
                "  {:>6}  {:─<6}  {:─<6}  {:─<5}  ← NOTE OFF",
                "───", "───", "───", "───"
            );
        }
        if should_print {
            let ratio = if sv > 0.005 {
                format!("{:5.2}", dv / sv)
            } else {
                "  -  ".to_string()
            };
            let bar_width = 32usize;
            let dh = ((dv * bar_width as f32) as usize).min(bar_width);
            let sh = ((sv * bar_width as f32) as usize).min(bar_width);
            let bar: String = (0..bar_width)
                .map(|i| match (i < dh, i < sh) {
                    (true, true) => '▓',
                    (true, false) => 'H',
                    (false, true) => 'S',
                    _ => ' ',
                })
                .collect();
            println!("  {:>6.0}  {:>6.3}  {:>6.3}  {}  {}", t, dv, sv, ratio, bar);
            prev_printed = true;
        } else {
            prev_printed = false;
        }
    }
}

// ── Comparison entry ──────────────────────────────────────────────────────────

fn compare_pair(dx100_path: &str, synth_path: &str, hold_ms: f32, verbose: bool) {
    let dx_bins = match load_rms_bins(dx100_path) {
        Some(b) => b,
        None => {
            println!("  cannot read {dx100_path}");
            return;
        }
    };
    let sy_bins = match load_rms_bins(synth_path) {
        Some(b) => b,
        None => {
            println!("  cannot read {synth_path}");
            return;
        }
    };

    let dx_peak = dx_bins.iter().cloned().fold(0.0_f32, f32::max);
    let sy_peak = sy_bins.iter().cloned().fold(0.0_f32, f32::max);
    if dx_peak < 1e-7 {
        println!("  HW: silent");
        return;
    }
    if sy_peak < 1e-7 {
        println!("  SY: silent");
        return;
    }

    let dx_onset = find_onset(&dx_bins);
    let sy_onset = find_onset(&sy_bins);
    let hold_bins = (hold_ms / WINDOW_MS) as usize;

    let dx_m = compute_metrics(&dx_bins, dx_onset, hold_bins);
    let sy_m = compute_metrics(&sy_bins, sy_onset, hold_bins);

    println!(
        "  atk(90%): HW={:7.1}ms  SY={:7.1}ms",
        dx_m.atk90_ms, sy_m.atk90_ms
    );
    println!("  d1l_lvl:  HW={:7.3}    SY={:7.3}", dx_m.d1l, sy_m.d1l);
    println!(
        "  rls(50%): HW={:7.1}ms  SY={:7.1}ms  (from note-off)",
        dx_m.rls50_ms, sy_m.rls50_ms
    );
    println!(
        "  rls(90%): HW={:7.1}ms  SY={:7.1}ms  (from note-off)",
        dx_m.rls90_ms, sy_m.rls90_ms
    );

    if verbose {
        print_envelope(&dx_bins, &sy_bins, dx_onset, sy_onset, hold_bins);
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let hold_ms: f32 = flag_val(&args, "--hold-ms")
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000.0);
    let detail_voice: Option<usize> = flag_val(&args, "--detail").and_then(|s| s.parse().ok());

    if let Some(dir) = flag_val(&args, "--dir") {
        // ── Batch mode ────────────────────────────────────────────────────────
        println!("=== EG Comparison  dir={dir}  hold={hold_ms:.0}ms ===");
        println!();
        println!(
            "{:<3}  {:<10}  {:>9}  {:>9}  {:>7}  {:>7}  {:>9}  {:>9}  {:>9}  {:>9}",
            "#",
            "Name",
            "atk90(HW)",
            "atk90(SY)",
            "d1l(HW)",
            "d1l(SY)",
            "rls50(HW)",
            "rls50(SY)",
            "rls90(HW)",
            "rls90(SY)"
        );
        println!("{}", "-".repeat(100));

        let mut dx_files: Vec<_> = std::fs::read_dir(format!("{dir}/dx100"))
            .expect("read dx100 dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |x| x == "wav"))
            .map(|e| e.path())
            .collect();
        dx_files.sort();

        for (idx, dx_path) in dx_files.iter().enumerate() {
            let fname = dx_path.file_name().unwrap().to_string_lossy();
            let sy_path_str = format!("{dir}/synth/{fname}");
            let voice_num = idx + 1;

            // Extract voice name (strip NN_ prefix and .wav suffix)
            let stem = dx_path.file_stem().unwrap().to_string_lossy();
            let name = stem.splitn(2, '_').nth(1).unwrap_or(&stem);

            let dx_bins = match load_rms_bins(&dx_path.to_string_lossy()) {
                Some(b) => b,
                None => {
                    println!("{:<3}  {:<10}  (cannot read)", voice_num, name);
                    continue;
                }
            };
            let sy_bins = match load_rms_bins(&sy_path_str) {
                Some(b) => b,
                None => {
                    println!("{:<3}  {:<10}  (no synth file)", voice_num, name);
                    continue;
                }
            };

            let dx_peak = dx_bins.iter().cloned().fold(0.0_f32, f32::max);
            let sy_peak = sy_bins.iter().cloned().fold(0.0_f32, f32::max);
            if dx_peak < 1e-7 || sy_peak < 1e-7 {
                println!("{:<3}  {:<10}  (silent)", voice_num, name);
                continue;
            }

            let dx_onset = find_onset(&dx_bins);
            let sy_onset = find_onset(&sy_bins);
            let hold_bins = (hold_ms / WINDOW_MS) as usize;
            let dm = compute_metrics(&dx_bins, dx_onset, hold_bins);
            let sm = compute_metrics(&sy_bins, sy_onset, hold_bins);

            println!(
                "{:<3}  {:<10}  {:>8.1}ms  {:>8.1}ms  {:>7.3}  {:>7.3}  {:>8.1}ms  {:>8.1}ms  {:>8.1}ms  {:>8.1}ms",
                voice_num, name,
                dm.atk90_ms, sm.atk90_ms,
                dm.d1l, sm.d1l,
                dm.rls50_ms, sm.rls50_ms,
                dm.rls90_ms, sm.rls90_ms,
            );

            if detail_voice == Some(voice_num) {
                println!();
                print_envelope(&dx_bins, &sy_bins, dx_onset, sy_onset, hold_bins);
                println!();
            }
        }
    } else if args.len() >= 2 && !args[0].starts_with('-') {
        // ── Single-pair verbose mode ──────────────────────────────────────────
        let dx100_path = &args[0];
        let synth_path = &args[1];
        let stem = std::path::Path::new(dx100_path)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        println!("=== {stem} (hold={hold_ms:.0}ms) ===");
        compare_pair(dx100_path, synth_path, hold_ms, true);
    } else {
        eprintln!("Usage:");
        eprintln!("  compare_eg --dir <dir> [--hold-ms <ms>] [--detail <voice_num>]");
        eprintln!("  compare_eg <dx100.wav> <synth.wav> [--hold-ms <ms>]");
    }
}

fn flag_val(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}
