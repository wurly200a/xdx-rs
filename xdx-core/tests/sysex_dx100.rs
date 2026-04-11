use xdx_core::sysex::{dx100_decode_1voice, dx100_encode_1voice};

static IVORY_EBONY_SYX: &[u8] = include_bytes!("../../testdata/syx/IvoryEbony.syx");

#[test]
fn decode_ivory_ebony_name() {
    let voice = dx100_decode_1voice(IVORY_EBONY_SYX).unwrap();
    assert_eq!(voice.name_str(), "IvoryEbony");
}

#[test]
fn decode_ivory_ebony_algorithm() {
    let voice = dx100_decode_1voice(IVORY_EBONY_SYX).unwrap();
    assert_eq!(voice.algorithm, 2); // ALGO 3 (0-indexed)
}

#[test]
fn decode_ivory_ebony_feedback() {
    let voice = dx100_decode_1voice(IVORY_EBONY_SYX).unwrap();
    assert_eq!(voice.feedback, 6);
}

#[test]
fn decode_ivory_ebony_transpose_center() {
    let voice = dx100_decode_1voice(IVORY_EBONY_SYX).unwrap();
    assert_eq!(voice.transpose, 24); // center = no transpose
}

#[test]
fn decode_ivory_ebony_op1_out_level() {
    let voice = dx100_decode_1voice(IVORY_EBONY_SYX).unwrap();
    assert_eq!(voice.ops[0].out_level, 99); // OP1 full output
}

#[test]
fn decode_ivory_ebony_op4_out_level() {
    let voice = dx100_decode_1voice(IVORY_EBONY_SYX).unwrap();
    assert_eq!(voice.ops[3].out_level, 65); // OP4
}

#[test]
fn decode_ivory_ebony_pitch_eg() {
    let voice = dx100_decode_1voice(IVORY_EBONY_SYX).unwrap();
    assert_eq!(voice.pitch_eg_rate,  [99, 99, 99]);
    assert_eq!(voice.pitch_eg_level, [50, 50, 50]);
}

#[test]
fn encode_decode_roundtrip() {
    let original = dx100_decode_1voice(IVORY_EBONY_SYX).unwrap();
    let encoded = dx100_encode_1voice(&original, 0);
    let decoded = dx100_decode_1voice(&encoded).unwrap();
    assert_eq!(original, decoded);
}

#[test]
fn encode_matches_original_bytes() {
    let voice = dx100_decode_1voice(IVORY_EBONY_SYX).unwrap();
    let encoded = dx100_encode_1voice(&voice, 0);
    assert_eq!(encoded, IVORY_EBONY_SYX);
}
