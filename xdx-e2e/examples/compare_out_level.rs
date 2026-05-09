/// Compare OUT LEVEL amplitude scaling between DX100 hardware and softsynth.
///
/// For each voice pair (dx100/*.wav vs synth/*.wav), computes the mean RMS over
/// the sustain window (20%-80% of the hold period), converts to dB, and reports
/// how closely it matches the expected 0.75 dB/step model.
///
/// Columns:
///   theory  – expected dB relative to OL=90 reference: (OL - 90) × 0.75
///   HW(dB)  – absolute RMS level of hardware recording
///   SY(dB)  – absolute RMS level of softsynth render
///   HW-thy  – (HW relative to ref) minus theory  (ideal = 0.00)
///   SY-thy  – (SY relative to ref) minus theory  (ideal = 0.00)
///   HW-SY   – HW relative minus SY relative      (ideal = 0.00)
///
/// Usage:
///   cargo run -p xdx-e2e --example compare_out_level -- --dir <dir> [--hold-ms <ms>]
///
/// Example:
///   cargo run -p xdx-e2e --example compare_out_level -- \
///     --dir calibration/out_level_calib --hold-ms 3000
use hound::WavReader;
use std::env;

const WINDOW_MS: f32 = 10.0;

fn load_rms_bins(path: &str) -> Option<Vec<f32>> {
    let mut reader = WavReader::open(path).ok()?;
    let sr = reader.spec().sample_rate as f32;
    let win = (sr * WINDOW_MS / 1000.0) as usize;
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();
    Some(
        samples
            .chunks(win)
            .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
            .collect(),
    )
}

fn find_onset(bins: &[f32]) -> usize {
    let peak = bins.iter().cloned().fold(0.0_f32, f32::max);
    let thr = peak * 0.005;
    bins.iter().position(|&r| r > thr).unwrap_or(0)
}

/// Mean RMS over the sustain window (20%–80% of hold period, post-onset).
fn sustain_rms(bins: &[f32], onset: usize, hold_bins: usize) -> f32 {
    let start = (onset + hold_bins / 5).min(bins.len());
    let end = (onset + hold_bins * 4 / 5).min(bins.len());
    if end <= start {
        return 0.0;
    }
    let sum: f32 = bins[start..end].iter().map(|&v| v * v).sum();
    (sum / (end - start) as f32).sqrt()
}

fn rms_to_db(rms: f32) -> f32 {
    if rms < 1e-9 {
        f32::NEG_INFINITY
    } else {
        20.0 * rms.log10()
    }
}

fn parse_ol(stem: &str) -> Option<u8> {
    // "NN_OL90" → 90
    let name = stem.splitn(2, '_').nth(1)?;
    name.strip_prefix("OL")?.parse().ok()
}

fn flag_val(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let hold_ms: f32 = flag_val(&args, "--hold-ms")
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000.0);
    let dir = match flag_val(&args, "--dir") {
        Some(d) => d,
        None => {
            eprintln!("Usage: compare_out_level --dir <dir> [--hold-ms <ms>]");
            std::process::exit(1);
        }
    };
    let hold_bins = (hold_ms / WINDOW_MS) as usize;

    let mut dx_files: Vec<_> = std::fs::read_dir(format!("{dir}/dx100"))
        .expect("read dx100/ dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "wav"))
        .map(|e| e.path())
        .collect();
    dx_files.sort();

    struct Row {
        idx: usize,
        name: String,
        ol: Option<u8>,
        hw_rms: f32,
        sy_rms: f32,
    }

    let mut rows: Vec<Row> = Vec::new();
    for (idx, dx_path) in dx_files.iter().enumerate() {
        let stem = dx_path.file_stem().unwrap().to_string_lossy().to_string();
        let name = stem.splitn(2, '_').nth(1).unwrap_or(&stem).to_string();
        let sy_path = format!(
            "{dir}/synth/{}",
            dx_path.file_name().unwrap().to_string_lossy()
        );

        let load_rms = |path: &str| -> f32 {
            load_rms_bins(path)
                .map(|bins| {
                    let onset = find_onset(&bins);
                    sustain_rms(&bins, onset, hold_bins)
                })
                .unwrap_or(0.0)
        };

        rows.push(Row {
            idx: idx + 1,
            name,
            ol: parse_ol(&stem),
            hw_rms: load_rms(&dx_path.to_string_lossy()),
            sy_rms: load_rms(&sy_path),
        });
    }

    // OL=90 voice as reference; fall back to first row
    let ref_row = rows
        .iter()
        .find(|r| r.ol == Some(90))
        .or_else(|| rows.first());
    let (ref_hw_rms, ref_sy_rms, ref_ol) = match ref_row {
        Some(r) => (r.hw_rms, r.sy_rms, r.ol.unwrap_or(90)),
        None => {
            eprintln!("no data found");
            return;
        }
    };
    let ref_hw_db = rms_to_db(ref_hw_rms);
    let ref_sy_db = rms_to_db(ref_sy_rms);

    println!("=== OUT LEVEL comparison  dir={dir}  hold={hold_ms:.0}ms ===");
    println!("reference: OL={ref_ol}  theory=(OL-{ref_ol})×0.75 dB  (ideal HW-thy=SY-thy=HW-SY=0)");
    println!();
    println!(
        "{:<3}  {:<8}  {:>2}  {:>7}  {:>8}  {:>8}  {:>7}  {:>7}  {:>7}",
        "#", "Name", "OL", "theory", "HW(dB)", "SY(dB)", "HW-thy", "SY-thy", "HW-SY"
    );
    println!("{}", "-".repeat(70));

    for r in &rows {
        let theory = r.ol.map(|ol| (ol as f32 - ref_ol as f32) * 0.75);
        let hw_db = rms_to_db(r.hw_rms);
        let sy_db = rms_to_db(r.sy_rms);
        let hw_rel = hw_db - ref_hw_db;
        let sy_rel = sy_db - ref_sy_db;

        let fmt_db = |v: f32| -> String {
            if v.is_infinite() {
                "  -inf  ".to_string()
            } else {
                format!("{:8.2}", v)
            }
        };
        let fmt_rel = |v: f32| -> String {
            if v.is_infinite() {
                "  -inf ".to_string()
            } else {
                format!("{:+7.2}", v)
            }
        };
        let fmt_err = |opt: Option<f32>| -> String {
            match opt {
                None => "   N/A ".to_string(),
                Some(f) => fmt_rel(f),
            }
        };

        let hw_err = theory.map(|th| hw_rel - th);
        let sy_err = theory.map(|th| sy_rel - th);

        println!(
            "{:<3}  {:<8}  {:>2}  {}  {}  {}  {}  {}  {}",
            r.idx,
            r.name,
            r.ol.unwrap_or(0),
            fmt_err(theory),
            fmt_db(hw_db),
            fmt_db(sy_db),
            fmt_err(hw_err),
            fmt_err(sy_err),
            fmt_rel(hw_rel - sy_rel),
        );
    }
}
