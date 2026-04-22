/// Generate two 1-voice sysex files for feedback calibration:
///   fb_test_0.syx  – identical voice with feedback = 0 (pure sine)
///   fb_test_7.syx  – identical voice with feedback = 7 (maximum self-modulation)
///
/// Voice design: OP1 carrier only, OP2-4 silenced.
/// Steady sustain (D1R=0, D2R=0) lets the feedback character ring clearly.
use xdx_core::dx100::{Dx100Operator, Dx100Voice};
use xdx_core::sysex::dx100_encode_1voice;

fn fb_test_voice(feedback: u8) -> Dx100Voice {
    let silent_op = Dx100Operator {
        ar: 31,
        d1r: 0,
        d2r: 0,
        rr: 7,
        d1l: 0,
        out_level: 0,
        freq_ratio: 4,
        detune: 3,
        kbd_lev_scl: 0,
        kbd_rate_scl: 0,
        eg_bias_sens: 0,
        amp_mod_en: 0,
        key_vel_sens: 0,
    };
    let carrier_op = Dx100Operator {
        out_level: 90,
        ..silent_op
    };

    let name_str = format!("FB TEST {:>3}", feedback);
    let mut name = [b' '; 10];
    for (i, b) in name_str.as_bytes().iter().take(10).enumerate() {
        name[i] = *b;
    }

    Dx100Voice {
        ops: [carrier_op, silent_op.clone(), silent_op.clone(), silent_op],
        algorithm: 0,
        feedback,
        transpose: 24,
        ..Default::default()
    }
}

fn main() {
    for fb in [0u8, 7u8] {
        let voice = fb_test_voice(fb);
        let bytes = dx100_encode_1voice(&voice, 0);
        let path = format!("fb_test_{fb}.syx");
        std::fs::write(&path, &bytes).expect("write failed");
        println!("wrote {path}  ({} bytes)", bytes.len());
    }
}
