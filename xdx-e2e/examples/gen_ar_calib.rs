/// Generate a 24-voice AR calibration bank for DX100 comparison.
///
/// All voices use algorithm 0 with OP1 as the sole carrier (pure sine, no modulation).
/// D1R=0 / D1L=15 holds the envelope at peak during note-on so only the attack phase
/// varies between voices.  RR=15 drops quickly on note-off.
///
/// Voice layout (slots 1-24):
///   AR = 1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,22,25,28,31
///
/// Slots 25-32 are silent padding to fill the 32-voice bank.
///
/// Fixed parameters: D1R=0, D1L=15, D2R=0, RR=15
///
/// Note on hold time: AR=1 reaches peak in ~7.9s.  Use record.json hold=10.0.
///
/// Writes to: calibration/eg_ar_calib/ar_calib.syx
///
/// Usage:
///   cargo run -p xdx-e2e --example gen_ar_calib
use xdx_core::dx100::{Dx100Operator, Dx100Voice, BANK_VOICES};
use xdx_core::sysex::dx100_encode_32voice;

fn make_voice(ar: u8, audible: bool) -> Dx100Voice {
    let op = Dx100Operator {
        ar,
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
    let carrier = Dx100Operator {
        out_level: if audible { 99 } else { 0 },
        ..op
    };

    let label = if audible {
        format!("AR{ar:02}")
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
    let ar_values: [u8; 24] = [
         1,  2,  3,  4,  5,  6,  7,  8,  9, 10,
        11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        22, 25, 28, 31,
    ];

    let mut voices: Vec<Dx100Voice> = ar_values.iter().map(|&ar| make_voice(ar, true)).collect();
    while voices.len() < BANK_VOICES {
        voices.push(make_voice(31, false));
    }

    println!(
        "{:<3}  {:<6}  {:>3}  {:>3}  {:>3}  {:>3}  {:>3}",
        "#", "Name", "AR", "D1R", "D1L", "D2R", "RR"
    );
    println!("{}", "-".repeat(34));
    for (i, v) in voices[..24].iter().enumerate() {
        let op = &v.ops[0];
        let label = String::from_utf8_lossy(&v.name).trim_end().to_string();
        println!(
            "{:<3}  {:<6}  {:>3}  {:>3}  {:>3}  {:>3}  {:>3}",
            i + 1,
            label,
            op.ar,
            op.d1r,
            op.d1l,
            op.d2r,
            op.rr
        );
    }

    let out_dir = "calibration/eg_ar_calib";
    std::fs::create_dir_all(out_dir).expect("create dir failed");
    let out_path = format!("{out_dir}/ar_calib.syx");
    let syx = dx100_encode_32voice(&voices, 0);
    std::fs::write(&out_path, &syx).expect("write failed");
    println!("\nWrote {out_path}  ({} bytes)", syx.len());
}
