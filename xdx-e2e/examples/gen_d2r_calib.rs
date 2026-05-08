/// Generate a 24-voice D2R calibration bank for DX100 comparison.
///
/// All voices use algorithm 0 with OP1 as the sole carrier (pure sine, no modulation).
/// AR=31 (instant attack), D1R=31, D1L=15 (peak): the D1 phase transitions to D2
/// in one sample, so D2R starts from level=1.0 immediately after the attack.
/// This isolates D2R exactly as gen_d1r_calib isolates D1R.
///
/// Voice layout (slots 1-24):
///   D2R = 1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,22,25,28,31
///
/// Slots 25-32 are silent padding to fill the 32-voice bank.
///
/// Fixed parameters: AR=31, D1R=31, D1L=15, RR=15
///
/// Key metric: dcy90_ms (peak → 10% level).
/// Expected half-lives are identical to D1R (same rate_mul formula):
///   0.000092 × 2^((31-D2R)×0.55) seconds
///
/// Use record.json hold=10.0.  D2R=1-3 will show NaN for dcy90 (decay incomplete).
///
/// Writes to: calibration/eg_d2r_calib/d2r_calib.syx
///
/// Usage:
///   cargo run -p xdx-e2e --example gen_d2r_calib
use xdx_core::dx100::{Dx100Operator, Dx100Voice, BANK_VOICES};
use xdx_core::sysex::dx100_encode_32voice;

fn make_voice(d2r: u8, audible: bool) -> Dx100Voice {
    let op = Dx100Operator {
        ar: 31,
        d1r: 31,
        d2r,
        rr: 15,
        d1l: 15,
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
        format!("D2R{d2r:02}")
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
    let d2r_values: [u8; 24] = [
         1,  2,  3,  4,  5,  6,  7,  8,  9, 10,
        11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        22, 25, 28, 31,
    ];

    let mut voices: Vec<Dx100Voice> = d2r_values
        .iter()
        .map(|&d2r| make_voice(d2r, true))
        .collect();
    while voices.len() < BANK_VOICES {
        voices.push(make_voice(1, false));
    }

    // half-life: 0.000092 * 2^((31-d2r)*0.55)  seconds  (same formula as D1R)
    // dcy90 = half-life * log2(10) ≈ half-life * 3.322  seconds
    let half_life_s = |d2r: u8| -> f32 { 0.000092_f32 * 2.0_f32.powf((31.0 - d2r as f32) * 0.55) };

    println!(
        "{:<3}  {:<6}  {:>3}  {:>3}  {:>3}  {:>3}  {:>3}  {:>9}  {:>9}",
        "#", "Name", "AR", "D1L", "D2R", "D1R", "RR", "hl(ms)", "dcy90(ms)"
    );
    println!("{}", "-".repeat(58));
    for (i, v) in voices[..24].iter().enumerate() {
        let op = &v.ops[0];
        let label = String::from_utf8_lossy(&v.name).trim_end().to_string();
        let hl = half_life_s(op.d2r) * 1000.0;
        let dcy90 = hl * 10.0_f32.log2();
        println!(
            "{:<3}  {:<6}  {:>3}  {:>3}  {:>3}  {:>3}  {:>3}  {:>9.1}  {:>9.1}",
            i + 1,
            label,
            op.ar,
            op.d1l,
            op.d2r,
            op.d1r,
            op.rr,
            hl,
            dcy90
        );
    }

    let out_dir = "calibration/eg_d2r_calib";
    std::fs::create_dir_all(out_dir).expect("create dir failed");
    let out_path = format!("{out_dir}/d2r_calib.syx");
    let syx = dx100_encode_32voice(&voices, 0);
    std::fs::write(&out_path, &syx).expect("write failed");
    println!("\nWrote {out_path}  ({} bytes)", syx.len());
}
