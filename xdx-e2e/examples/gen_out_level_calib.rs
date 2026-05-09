/// Generate a 24-voice OUT LEVEL calibration bank for DX100 comparison.
///
/// All voices use algorithm 0 with OP1 as the sole carrier (pure sine, no modulation).
/// EG holds at full level throughout the note (AR=31, D1R=0, D1L=15, D2R=0, RR=15),
/// so the sustain-phase amplitude is determined entirely by OUT_LEVEL.
///
/// OL=91-99 are excluded per the DX100 manual (distortion above 90).
/// OL=0 is excluded because level_to_amp(0) returns 0.0 (silence).
///
/// Voice layout (slots 1-24):
///   Group A (1-12):  OL = 90, 86, 82, 78, 74, 70, 66, 62, 58, 54, 50, 46  (step=4)
///   Group B (13-24): OL = 42, 38, 34, 30, 26, 22, 18, 14, 10,  6,  3,  1
///
/// Expected amplitude ratio between adjacent 4-step voices (3.0 dB):
///   amp(OL-4) / amp(OL) = 10^(-3.0/20) ≈ 0.708
///
/// Writes to: calibration/out_level_calib/out_level_calib.syx
///
/// Usage:
///   cargo run -p xdx-e2e --example gen_out_level_calib
use xdx_core::dx100::{Dx100Operator, Dx100Voice, BANK_VOICES};
use xdx_core::sysex::dx100_encode_32voice;

fn make_voice(out_level: u8) -> Dx100Voice {
    let op = Dx100Operator {
        ar: 31,
        d1r: 0,
        d2r: 0,
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
    let carrier = Dx100Operator { out_level, ..op };

    let label = format!("OL{out_level:02}");
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
    let ol_values: [u8; BANK_VOICES] = [
        // Group A: OL 90..46, step=4 (12 voices)
        90, 86, 82, 78, 74, 70, 66, 62, 58, 54, 50, 46,
        // Group B: OL 42..1 (12 voices)
        42, 38, 34, 30, 26, 22, 18, 14, 10,  6,  3,  1,
    ];

    let voices: Vec<Dx100Voice> = ol_values.iter().map(|&ol| make_voice(ol)).collect();

    let theory_db = |ol: u8| -> f32 { (ol as f32 - 90.0) * 0.75 };
    let theory_amp = |ol: u8| -> f32 { 10.0_f32.powf(theory_db(ol) / 20.0) };

    println!(
        "{:<3}  {:<8}  {:>2}  {:>8}  {:>8}",
        "#", "Name", "OL", "dB(rel)", "amp(rel)"
    );
    println!("{}", "-".repeat(38));
    for (i, (&ol, v)) in ol_values.iter().zip(voices.iter()).enumerate() {
        let label = String::from_utf8_lossy(&v.name).trim_end().to_string();
        println!(
            "{:<3}  {:<8}  {:>2}  {:>+8.2}  {:>8.5}",
            i + 1,
            label,
            ol,
            theory_db(ol),
            theory_amp(ol)
        );
    }

    let out_dir = "calibration/out_level_calib";
    std::fs::create_dir_all(out_dir).expect("create dir failed");
    let out_path = format!("{out_dir}/out_level_calib.syx");
    let syx = dx100_encode_32voice(&voices, 0);
    std::fs::write(&out_path, &syx).expect("write failed");
    println!("\nWrote {out_path}  ({} bytes)", syx.len());
}
