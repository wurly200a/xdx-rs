//! Numerical analysis of kbs_calib recordings.
//! Derives kbd_lev_scl / kbd_rate_scl normalizer coefficients from DX100 hardware.
//!
//! Usage:
//!   cargo run -p xdx-compare --bin analyze-kbs-calib --release -- [--dir out/kbs_calib]

use hound::WavReader;

const WIN_MS: f32 = 10.0;
const HOLD_MS: f32 = 2000.0;
const DB_PER_STEP: f32 = 0.75; // out_level dB/step (Yamaha spec)

// ── WAV helpers ───────────────────────────────────────────────────────────────

fn load_rms(path: &str) -> Option<Vec<f32>> {
    let mut reader = WavReader::open(path).ok()?;
    let sr = reader.spec().sample_rate as f32;
    let win = (sr * WIN_MS / 1000.0) as usize;
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
    bins.iter().position(|&r| r > peak * 0.005).unwrap_or(0)
}

/// Average RMS over the middle 80% of the hold period.
fn hold_rms(bins: &[f32], onset: usize) -> f32 {
    let hold_bins = (HOLD_MS / WIN_MS) as usize;
    let s = onset + hold_bins / 10;
    let e = onset + hold_bins * 9 / 10;
    let slice = &bins[s.min(bins.len())..e.min(bins.len())];
    if slice.is_empty() {
        return 0.0;
    }
    slice.iter().copied().sum::<f32>() / slice.len() as f32
}

/// Time (ms) from the RMS peak to first crossing below 50% of that peak.
/// Measures the rate_mul half-life: t = coeff * 2^((max-rate)*0.55).
/// Searching from the peak (not onset) avoids returning 0 when onset is mid-attack.
fn decay_t50(bins: &[f32], onset: usize) -> f32 {
    let hold_bins = (HOLD_MS / WIN_MS) as usize;
    let end = (onset + hold_bins).min(bins.len().saturating_sub(1));

    // Find peak position within hold window
    let peak_pos = (onset..=end)
        .max_by(|&a, &b| {
            bins[a]
                .partial_cmp(&bins[b])
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(onset);
    let peak = bins[peak_pos];
    if peak < 1e-7 {
        return f32::NAN;
    }

    // Search for 50% crossing AFTER the peak
    let thr = peak * 0.5;
    (peak_pos..=end)
        .find(|&i| bins[i] < thr)
        .map(|i| (i - peak_pos) as f32 * WIN_MS)
        .unwrap_or(f32::NAN)
}

/// Implied krs rate_boost from measured t50 ratio (faster/slower).
/// Half-life t ∝ 2^((max-rate)*0.55), so ratio = 2^(-boost*0.55)
/// → boost = -log2(ratio) / 0.55
fn implied_boost(ratio: f32) -> f32 {
    if ratio <= 0.0 || ratio.is_nan() {
        return f32::NAN;
    }
    -ratio.log2() / 0.55
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let base_dir = flag_val(&args, "--dir").unwrap_or_else(|| "out/kbs_calib".to_string());

    // note tag, semitones offset from breakpoint (C4=60)
    let notes: &[(&str, i32)] = &[("n48", -12), ("n60", 0), ("n72", 12), ("n84", 24)];

    // Group A: kls sweep (voices 0-3)
    let a_names = ["SUST_BASE", "KLS_025", "KLS_050", "KLS_099"];
    let a_files = [
        "01_SUST_BASE.wav",
        "02_KLS_025.wav",
        "03_KLS_050.wav",
        "04_KLS_099.wav",
    ];
    let a_kls: [u8; 4] = [0, 25, 50, 99];

    // Group B: krs sweep (voices 4-7)
    let b_names = ["DCY_BASE", "KRS1_D10", "KRS2_D10", "KRS3_D10"];
    let b_files = [
        "05_DCY_BASE.wav",
        "06_KRS1_D10.wav",
        "07_KRS2_D10.wav",
        "08_KRS3_D10.wav",
    ];
    let b_krs: [u8; 4] = [0, 1, 2, 3];

    // Load all envelopes: data_a[note_idx][voice_idx], data_b[note_idx][voice_idx]
    let data_a: Vec<Vec<Option<Vec<f32>>>> = notes
        .iter()
        .map(|&(ntag, _)| {
            a_files
                .iter()
                .map(|fname| {
                    let path = format!("{base_dir}/{ntag}/dx100/{fname}");
                    let v = load_rms(&path);
                    if v.is_none() {
                        eprintln!("  missing: {path}");
                    }
                    v
                })
                .collect()
        })
        .collect();

    let data_b: Vec<Vec<Option<Vec<f32>>>> = notes
        .iter()
        .map(|&(ntag, _)| {
            b_files
                .iter()
                .map(|fname| {
                    let path = format!("{base_dir}/{ntag}/dx100/{fname}");
                    let v = load_rms(&path);
                    if v.is_none() {
                        eprintln!("  missing: {path}");
                    }
                    v
                })
                .collect()
        })
        .collect();

    // Compute metrics
    let a_rms: Vec<[f32; 4]> = (0..notes.len())
        .map(|ni| {
            std::array::from_fn(|vi| {
                data_a[ni][vi]
                    .as_ref()
                    .map(|b| hold_rms(b, find_onset(b)))
                    .unwrap_or(0.0)
            })
        })
        .collect();

    let b_t50: Vec<[f32; 4]> = (0..notes.len())
        .map(|ni| {
            std::array::from_fn(|vi| {
                data_b[ni][vi]
                    .as_ref()
                    .map(|b| decay_t50(b, find_onset(b)))
                    .unwrap_or(f32::NAN)
            })
        })
        .collect();

    // ── Group A: kbd_lev_scl ──────────────────────────────────────────────────
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  kbd_lev_scl  (Group A: sustained, D1R=0, D1L=15)       ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    println!("── Hold RMS (absolute) ──");
    print!("{:<14}", "Voice");
    for &(ntag, off) in notes {
        print!("  {:>15}", format!("{ntag}({off:+}st)"));
    }
    println!();
    for vi in 0..4 {
        print!("{:<14}", a_names[vi]);
        for ni in 0..notes.len() {
            print!("  {:>15.5}", a_rms[ni][vi]);
        }
        println!();
    }

    println!();
    println!("── dB reduction vs SUST_BASE (same note)  |  implied steps ──");
    print!("{:<16}", "Voice(kls)");
    for &(ntag, off) in notes {
        print!("  {:>15}", format!("{ntag}({off:+}st)"));
    }
    println!();

    let mut kls_norms: Vec<f32> = Vec::new();
    // kls_meas[vi_off][ni] = implied reduction steps
    let mut kls_meas: [[f32; 4]; 3] = [[0.0; 4]; 3];

    for vi_off in 0..3 {
        let vi = vi_off + 1;
        let kls = a_kls[vi];
        print!("{:<16}", format!("{}(k={kls})", a_names[vi]));
        for (ni, &(_, off)) in notes.iter().enumerate() {
            let base = a_rms[ni][0];
            let kv = a_rms[ni][vi];
            if base > 1e-7 {
                let db = 20.0 * (kv / base).log10();
                let steps = (-db / DB_PER_STEP).max(0.0);
                kls_meas[vi_off][ni] = steps;
                print!("  {:>7.2}dB ({:>4.1}st)", db, steps);
                if off > 0 && steps > 0.5 {
                    kls_norms.push(kls as f32 * off as f32 / steps);
                }
            } else {
                print!("  {:>15}", "N/A");
            }
        }
        println!();
    }

    let note_midis: [u8; 4] = [48, 60, 72, 84];
    println!();
    println!("── New formula (kls * 2^(note/12) / 400) [steps]  vs  measured ──");
    print!("{:<16}", "Voice(kls)");
    for &(ntag, off) in notes {
        print!("  {:>15}", format!("{ntag}({off:+}st)"));
    }
    println!();
    for vi_off in 0..3 {
        let vi = vi_off + 1;
        let kls = a_kls[vi];
        print!("{:<16}", format!("{}(k={kls})", a_names[vi]));
        for ni in 0..notes.len() {
            let note_midi = note_midis[ni];
            let pred = (kls as f32 * 2.0_f32.powf(note_midi as f32 / 12.0) / 400.0) as u8 as f32;
            let meas = kls_meas[vi_off][ni];
            let mark = if (meas - pred).abs() < 1.0 {
                "✓"
            } else if (meas - pred).abs() < 2.0 {
                "△"
            } else {
                "✗"
            };
            print!("  {:>6.0}st({:>4.1}meas){}", pred, meas, mark);
        }
        println!();
    }

    // ── Group B: kbd_rate_scl ─────────────────────────────────────────────────
    println!();
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  kbd_rate_scl  (Group B: decaying, D1R=10, D1L=0)       ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    println!("── Decay t50 (ms from onset to 50% of peak) ──");
    print!("{:<14}", "Voice");
    for &(ntag, off) in notes {
        print!("  {:>15}", format!("{ntag}({off:+}st)"));
    }
    println!();
    for vi in 0..4 {
        print!("{:<14}", b_names[vi]);
        for ni in 0..notes.len() {
            let t = b_t50[ni][vi];
            if t.is_nan() {
                print!("  {:>15}", "N/A");
            } else {
                print!("  {:>13.0}ms", t);
            }
        }
        println!();
    }

    println!();
    println!("── t50 ratio vs DCY_BASE (same note)  |  implied boost ──");
    print!("{:<16}", "Voice(krs)");
    for &(ntag, off) in notes {
        print!("  {:>16}", format!("{ntag}({off:+}st)"));
    }
    println!();

    let mut krs_norms: Vec<f32> = Vec::new();
    let mut krs_meas_boost: [[f32; 4]; 3] = [[f32::NAN; 4]; 3];

    for vi_off in 0..3 {
        let vi = vi_off + 1;
        let krs = b_krs[vi];
        print!("{:<16}", format!("{}(k={krs})", b_names[vi]));
        for (ni, &(_, off)) in notes.iter().enumerate() {
            let tb = b_t50[ni][0];
            let tk = b_t50[ni][vi];
            if !tb.is_nan() && !tk.is_nan() && tb > 0.0 {
                let ratio = tk / tb;
                let boost = implied_boost(ratio);
                krs_meas_boost[vi_off][ni] = boost;
                print!("  {:>8.3}({:>5.2}b)", ratio, boost);
                if off > 0 && !boost.is_nan() && boost > 0.0 {
                    krs_norms.push(krs as f32 * off as f32 / boost);
                }
            } else {
                print!("  {:>16}", "N/A");
            }
        }
        println!();
    }

    println!();
    println!("── New formula (tri(krs)*note/72) [boost]  vs  measured  |  expected t50 ratio ──");
    println!("   tri(krs) = krs*(krs+1)/2 → [0,1,3,6]; norm=72 (C5)");
    print!("{:<16}", "Voice(krs)");
    for &(ntag, off) in notes {
        print!("  {:>20}", format!("{ntag}({off:+}st)"));
    }
    println!();
    for vi_off in 0..3 {
        let vi = vi_off + 1;
        let krs = b_krs[vi];
        let effective_krs = (krs * (krs + 1)) / 2; // triangular: [0,1,3,6]
        print!("{:<16}", format!("{}(k={krs})", b_names[vi]));
        for (ni, &(_, _off)) in notes.iter().enumerate() {
            let note_midi = note_midis[ni];
            let pred = (effective_krs as f32 * note_midi as f32 / 72.0).round();
            let meas = krs_meas_boost[vi_off][ni];
            let exp_r = 2.0_f32.powf(-pred * 0.55);
            let mark = if !meas.is_nan() {
                if (meas - pred).abs() < 0.5 {
                    "✓"
                } else if (meas - pred).abs() < 1.5 {
                    "△"
                } else {
                    "✗"
                }
            } else {
                " "
            };
            print!("  {:>5.1}b({:>4.1}meas){} r={:.3}", pred, meas, mark, exp_r);
        }
        println!();
    }
}

fn flag_val(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}
