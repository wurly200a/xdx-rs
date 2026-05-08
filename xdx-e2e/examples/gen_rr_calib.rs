/// Generate a 15-voice RR calibration bank for DX100 comparison.
///
/// All voices use algorithm 0 with OP1 as the sole carrier (pure sine, no modulation).
/// AR=31 (instant attack), D1R=0, D1L=15, D2R=0: the envelope holds at level=1.0
/// throughout the entire note-on period, so RR starts from level=1.0 at note-off.
///
/// Voice layout (slots 1-15):
///   RR = 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15
///
/// Slots 16-32 are silent padding to fill the 32-voice bank.
///
/// Fixed parameters: AR=31, D1R=0, D1L=15, D2R=0
///
/// Key metric: rls90_ms (note-off → 10% of note-off level).
/// Expected half-lives (SW model: 0.0014 × 2^((15-RR)×0.55)):
///   RR=1  → hl≈294 ms → rls90≈ 976 ms
///   RR=5  → hl≈ 63 ms → rls90≈ 210 ms
///   RR=10 → hl≈  9 ms → rls90≈  31 ms
///   RR=15 → hl≈1.4 ms → rls90≈   5 ms
///
/// Note: RR max is 15 (vs 31 for D1R/D2R); coefficient is 0.0014 (vs 0.000092).
/// All values complete within release=2.0 s.
///
/// Writes to: calibration/eg_rr_calib/rr_calib.syx
///
/// Usage:
///   cargo run -p xdx-e2e --example gen_rr_calib
use xdx_core::dx100::{Dx100Operator, Dx100Voice, BANK_VOICES};
use xdx_core::sysex::dx100_encode_32voice;

fn make_voice(rr: u8, audible: bool) -> Dx100Voice {
    let op = Dx100Operator {
        ar: 31,
        d1r: 0,
        d2r: 0,
        rr,
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
        format!("RR{rr:02}")
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
    let rr_values: [u8; 15] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

    let mut voices: Vec<Dx100Voice> = rr_values.iter().map(|&rr| make_voice(rr, true)).collect();
    while voices.len() < BANK_VOICES {
        voices.push(make_voice(1, false));
    }

    // half-life: 0.0014 * 2^((15-rr)*0.55)  seconds
    // rls90 = half-life * log2(10) ≈ half-life * 3.322  seconds
    let half_life_s = |rr: u8| -> f32 { 0.0014_f32 * 2.0_f32.powf((15.0 - rr as f32) * 0.55) };

    println!(
        "{:<3}  {:<5}  {:>3}  {:>3}  {:>3}  {:>3}  {:>2}  {:>9}  {:>9}",
        "#", "Name", "AR", "D1R", "D1L", "D2R", "RR", "hl(ms)", "rls90(ms)"
    );
    println!("{}", "-".repeat(56));
    for (i, v) in voices[..15].iter().enumerate() {
        let op = &v.ops[0];
        let label = String::from_utf8_lossy(&v.name).trim_end().to_string();
        let hl = half_life_s(op.rr) * 1000.0;
        let rls90 = hl * 10.0_f32.log2();
        println!(
            "{:<3}  {:<5}  {:>3}  {:>3}  {:>3}  {:>3}  {:>2}  {:>9.1}  {:>9.1}",
            i + 1,
            label,
            op.ar,
            op.d1r,
            op.d1l,
            op.d2r,
            op.rr,
            hl,
            rls90
        );
    }

    let out_dir = "calibration/eg_rr_calib";
    std::fs::create_dir_all(out_dir).expect("create dir failed");
    let out_path = format!("{out_dir}/rr_calib.syx");
    let syx = dx100_encode_32voice(&voices, 0);
    std::fs::write(&out_path, &syx).expect("write failed");
    println!("\nWrote {out_path}  ({} bytes)", syx.len());
}
