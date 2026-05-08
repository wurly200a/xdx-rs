/// Generate a 24-voice D1R calibration bank for DX100 comparison.
///
/// All voices use algorithm 0 with OP1 as the sole carrier (pure sine, no modulation).
/// AR=31 (instant attack) and D1L=0 (decay to silence) isolate the D1R decay rate.
/// D2R=0 holds the envelope at silence after D1R completes.
///
/// Voice layout (slots 1-24):
///   D1R = 1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,22,25,28,31
///
/// Slots 25-32 are silent padding to fill the 32-voice bank.
///
/// Fixed parameters: AR=31, D1L=0, D2R=0, RR=15
///
/// Key metric: dcy90_ms (peak → 10% level).
/// Expected half-lives (SW model: 0.000092 × 2^((31-D1R)×0.55)):
///   D1R=1  → hl≈8.5 s  → dcy90≈28 s  (NaN in 10 s window)
///   D1R=4  → hl≈2.7 s  → dcy90≈ 9 s  (just within 10 s window)
///   D1R=5  → hl≈1.8 s  → dcy90≈ 6 s
///   D1R=10 → hl≈0.28 s → dcy90≈ 0.9 s
///   D1R=31 → hl≈0.09 ms → dcy90≈ 0.3 ms
///
/// Use record.json hold=10.0.  D1R=1-3 will show NaN for dcy90 (decay incomplete).
///
/// Writes to: calibration/eg_d1r_calib/d1r_calib.syx
///
/// Usage:
///   cargo run -p xdx-e2e --example gen_d1r_calib
use xdx_core::dx100::{Dx100Operator, Dx100Voice, BANK_VOICES};
use xdx_core::sysex::dx100_encode_32voice;

fn make_voice(d1r: u8, audible: bool) -> Dx100Voice {
    let op = Dx100Operator {
        ar: 31,
        d1r,
        d2r: 0,
        rr: 15,
        d1l: 0,
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
        format!("D1R{d1r:02}")
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
    #[rustfmt::skip]
    let d1r_values: [u8; 24] = [
         1,  2,  3,  4,  5,  6,  7,  8,  9, 10,
        11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        22, 25, 28, 31,
    ];

    let mut voices: Vec<Dx100Voice> = d1r_values
        .iter()
        .map(|&d1r| make_voice(d1r, true))
        .collect();
    while voices.len() < BANK_VOICES {
        voices.push(make_voice(1, false));
    }

    // half-life: 0.000092 * 2^((31-d1r)*0.55)  seconds
    // dcy90 = half-life * log2(10) ≈ half-life * 3.322  seconds
    let half_life_s = |d1r: u8| -> f32 { 0.000092_f32 * 2.0_f32.powf((31.0 - d1r as f32) * 0.55) };

    println!(
        "{:<3}  {:<6}  {:>3}  {:>3}  {:>3}  {:>3}  {:>3}  {:>9}  {:>9}",
        "#", "Name", "AR", "D1R", "D1L", "D2R", "RR", "hl(ms)", "dcy90(ms)"
    );
    println!("{}", "-".repeat(58));
    for (i, v) in voices[..24].iter().enumerate() {
        let op = &v.ops[0];
        let label = String::from_utf8_lossy(&v.name).trim_end().to_string();
        let hl = half_life_s(op.d1r) * 1000.0;
        let dcy90 = hl * 10.0_f32.log2();
        println!(
            "{:<3}  {:<6}  {:>3}  {:>3}  {:>3}  {:>3}  {:>3}  {:>9.1}  {:>9.1}",
            i + 1,
            label,
            op.ar,
            op.d1r,
            op.d1l,
            op.d2r,
            op.rr,
            hl,
            dcy90
        );
    }

    let out_dir = "calibration/eg_d1r_calib";
    std::fs::create_dir_all(out_dir).expect("create dir failed");
    let out_path = format!("{out_dir}/d1r_calib.syx");
    let syx = dx100_encode_32voice(&voices, 0);
    std::fs::write(&out_path, &syx).expect("write failed");
    println!("\nWrote {out_path}  ({} bytes)", syx.len());
}
