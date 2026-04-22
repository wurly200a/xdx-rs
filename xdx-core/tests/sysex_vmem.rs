use xdx_core::sysex::{dx100_decode_32voice, dx100_encode_32voice};

static ALL_VOICES_SYX: &[u8] = include_bytes!("../../testdata/syx/all_voices.syx");

#[test]
fn vmem_roundtrip_byte_identical() {
    let voices = dx100_decode_32voice(ALL_VOICES_SYX).unwrap();
    let channel = ALL_VOICES_SYX[2] & 0x0F;
    let encoded = dx100_encode_32voice(&voices, channel);
    if encoded != ALL_VOICES_SYX {
        for (i, (&a, &b)) in ALL_VOICES_SYX.iter().zip(encoded.iter()).enumerate() {
            if a != b {
                println!(
                    "byte[{i}]: original=0x{a:02X} encoded=0x{b:02X}  (voice {}, vmem_off {})",
                    if i >= 6 { (i - 6) / 128 } else { 0 },
                    if i >= 6 { (i - 6) % 128 } else { 0 }
                );
            }
        }
        panic!("VMEM decode→encode roundtrip is NOT byte-identical");
    }
}
