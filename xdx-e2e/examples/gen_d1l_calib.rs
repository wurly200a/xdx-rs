/// Generate a 15-voice D1L calibration bank for DX100 comparison.
///
/// All voices use algorithm 0 with OP1 as the sole carrier (pure sine, no modulation).
/// AR=31 (instant attack) and D2R=0 (hold at D1L forever) isolate the D1L sustain level.
/// D1R=10 decays to D1L within ~2 s (half-life ≈0.276 s), leaving a clean plateau
/// for the last 10% of the 4 s hold window used by compare_eg.
///
/// Voice layout (slots 1-15):
///   D1L = 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15
///
/// Slots 16-32 are silent padding to fill the 32-voice bank.
///
/// Fixed parameters: AR=31, D1R=10, D2R=0, RR=10
///
/// Expected SW levels (3 dB per step, 2^((D1L-15)/2)):
///   D1L=15 → 1.000 (0 dB), D1L=14 → 0.707 (-3 dB), D1L=13 → 0.500 (-6 dB), …
///   D1L=1  → 0.0078 (-42 dB)
///
/// Note on hold time: D1R=10 half-life ≈0.276 s; D1L=1 settles in ~1.9 s.
/// Use record.json hold=4.0 so compare_eg d1l window (last 10%) starts at 3.6 s.
///
/// Writes to: calibration/eg_d1l_calib/d1l_calib.syx
///
/// Usage:
///   cargo run -p xdx-e2e --example gen_d1l_calib
use xdx_core::dx100::{Dx100Operator, Dx100Voice, BANK_VOICES};
use xdx_core::sysex::dx100_encode_32voice;

fn make_voice(d1l: u8, audible: bool) -> Dx100Voice {
    let op = Dx100Operator {
        ar: 31,
        d1r: 10,
        d2r: 0,
        rr: 10,
        d1l,
        out_level: 0,
        freq_ratio: 4, // ×1.0
        detune: 3,     // center
        kbd_lev_scl: 0,
        kbd_rate_scl: 0,
        eg_bias_sens: 0,
        amp_mod_en: 0,
        key_vel_sens: 0,
    };
    let carrier = Dx100Operator {
        out_level: if audible { 99 } else { 0 },
        ..op
    };

    let label = if audible {
        format!("D1L{d1l:02}")
    } else {
        "PAD".to_string()
    };
    let mut name = [b' '; 10];
    for (i, b) in label.as_bytes().iter().take(10).enumerate() {
        name[i] = *b;
    }

    Dx100Voice {
        ops: [carrier, op.clone(), op.clone(), op],
        algorithm: 0,
        feedback: 0,
        transpose: 24,
        name,
        ..Default::default()
    }
}

fn main() {
    let d1l_values: [u8; 15] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

    let mut voices: Vec<Dx100Voice> = d1l_values
        .iter()
        .map(|&d1l| make_voice(d1l, true))
        .collect();
    while voices.len() < BANK_VOICES {
        voices.push(make_voice(15, false));
    }

    println!(
        "{:<3}  {:<6}  {:>3}  {:>3}  {:>3}  {:>3}  {:>3}  {:>8}",
        "#", "Name", "AR", "D1R", "D1L", "D2R", "RR", "Exp.level"
    );
    println!("{}", "-".repeat(46));
    for (i, v) in voices[..15].iter().enumerate() {
        let op = &v.ops[0];
        let label = String::from_utf8_lossy(&v.name).trim_end().to_string();
        let expected = if op.d1l == 0 {
            0.0_f32
        } else if op.d1l >= 15 {
            1.0_f32
        } else {
            2.0_f32.powf((op.d1l as f32 - 15.0) * 0.5)
        };
        println!(
            "{:<3}  {:<6}  {:>3}  {:>3}  {:>3}  {:>3}  {:>3}  {:>8.4}",
            i + 1,
            label,
            op.ar,
            op.d1r,
            op.d1l,
            op.d2r,
            op.rr,
            expected
        );
    }

    let out_dir = "calibration/eg_d1l_calib";
    std::fs::create_dir_all(out_dir).expect("create dir failed");
    let out_path = format!("{out_dir}/d1l_calib.syx");
    let syx = dx100_encode_32voice(&voices, 0);
    std::fs::write(&out_path, &syx).expect("write failed");
    println!("\nWrote {out_path}  ({} bytes)", syx.len());
}
