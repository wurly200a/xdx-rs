use crate::dx7::Dx7Voice;
use crate::dx100::{Dx100Operator, Dx100Voice};

#[derive(Debug, PartialEq)]
pub enum SysExError {
    TooShort,
    InvalidHeader,
    InvalidFooter,
    InvalidByteCount { expected: u16, actual: u16 },
    ChecksumMismatch { expected: u8, actual: u8 },
}

// ── DX100 1-voice bulk dump (101 bytes total) ─────────────────────────────────
// F0 43 0n 03 00 5D <93 bytes payload> <checksum> F7
//
// Payload layout (93 bytes):
//   [00-12]  OP4 params
//   [13-25]  OP2 params  (note: order is OP4, OP2, OP3, OP1)
//   [26-38]  OP3 params
//   [39-51]  OP1 params
//   [52]     algorithm
//   [53]     feedback
//   [54]     lfo_speed
//   [55]     lfo_delay
//   [56]     lfo_pmd
//   [57]     lfo_amd
//   [58]     lfo_sync
//   [59]     lfo_wave
//   [60]     pitch_mod_sens
//   [61]     amp_mod_sens
//   [62]     transpose
//   [63]     poly_mono
//   [64]     pb_range
//   [65]     porta_mode
//   [66]     porta_time
//   [67]     fc_volume
//   [68]     sustain
//   [69]     portamento (foot sw)
//   [70]     chorus
//   [71]     mw_pitch
//   [72]     mw_amplitude
//   [73]     bc_pitch
//   [74]     bc_amplitude
//   [75]     bc_pitch_bias
//   [76]     bc_eg_bias
//   [77-86]  name (10 ASCII bytes)
//   [87-89]  pitch_eg_rate[0..2]
//   [90-92]  pitch_eg_level[0..2]
//
// Each operator block (13 bytes):
//   [+0] ar, [+1] d1r, [+2] d2r, [+3] rr, [+4] d1l,
//   [+5] kbd_lev_scl, [+6] kbd_rate_scl, [+7] eg_bias_sens,
//   [+8] amp_mod_en, [+9] key_vel_sens, [+10] out_level,
//   [+11] freq_ratio, [+12] detune

const DX100_1VOICE_PAYLOAD_LEN: u16 = 93; // 0x5D
const DX100_1VOICE_TOTAL_LEN:   usize = 101;

pub fn dx100_decode_1voice(data: &[u8]) -> Result<Dx100Voice, SysExError> {
    if data.len() < DX100_1VOICE_TOTAL_LEN {
        return Err(SysExError::TooShort);
    }
    if data[0] != 0xF0 || data[1] != 0x43 || data[3] != 0x03 {
        return Err(SysExError::InvalidHeader);
    }
    if data[DX100_1VOICE_TOTAL_LEN - 1] != 0xF7 {
        return Err(SysExError::InvalidFooter);
    }
    let byte_count = ((data[4] as u16) << 7) | (data[5] as u16);
    if byte_count != DX100_1VOICE_PAYLOAD_LEN {
        return Err(SysExError::InvalidByteCount {
            expected: DX100_1VOICE_PAYLOAD_LEN,
            actual: byte_count,
        });
    }
    let payload = &data[6..6 + DX100_1VOICE_PAYLOAD_LEN as usize];
    verify_checksum(payload, data[6 + DX100_1VOICE_PAYLOAD_LEN as usize])?;
    Ok(parse_dx100_1voice(payload))
}

pub fn dx100_encode_1voice(voice: &Dx100Voice, channel: u8) -> Vec<u8> {
    let payload = build_dx100_1voice(voice);
    let checksum = calc_checksum(&payload);
    let mut out = vec![
        0xF0, 0x43, 0x00 | (channel & 0x0F), 0x03,
        0x00, 0x5D,
    ];
    out.extend_from_slice(&payload);
    out.push(checksum);
    out.push(0xF7);
    out
}

// ── DX7 1-voice bulk dump (155 bytes total) ───────────────────────────────────
// F0 43 0n 00 01 1B <128 bytes payload> <checksum> F7

pub fn dx7_decode_1voice(_data: &[u8]) -> Result<Dx7Voice, SysExError> {
    todo!("DX7 1-voice decode not yet implemented")
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn calc_checksum(data: &[u8]) -> u8 {
    let sum: u8 = data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    ((!sum).wrapping_add(1)) & 0x7F
}

fn verify_checksum(data: &[u8], expected: u8) -> Result<(), SysExError> {
    let actual = calc_checksum(data);
    if actual != expected {
        Err(SysExError::ChecksumMismatch { expected, actual })
    } else {
        Ok(())
    }
}

fn parse_op(b: &[u8]) -> Dx100Operator {
    Dx100Operator {
        ar:           b[0],
        d1r:          b[1],
        d2r:          b[2],
        rr:           b[3],
        d1l:          b[4],
        kbd_lev_scl:  b[5],
        kbd_rate_scl: b[6],
        eg_bias_sens: b[7],
        amp_mod_en:   b[8],
        key_vel_sens: b[9],
        out_level:    b[10],
        freq_ratio:   b[11],
        detune:       b[12],
    }
}

fn parse_dx100_1voice(p: &[u8]) -> Dx100Voice {
    // SysEx order: OP4(0), OP2(13), OP3(26), OP1(39)
    // Store as ops[0]=OP1, ops[1]=OP2, ops[2]=OP3, ops[3]=OP4
    let op4 = parse_op(&p[0..]);
    let op2 = parse_op(&p[13..]);
    let op3 = parse_op(&p[26..]);
    let op1 = parse_op(&p[39..]);

    Dx100Voice {
        ops:            [op1, op2, op3, op4],
        algorithm:      p[52],
        feedback:       p[53],
        lfo_speed:      p[54],
        lfo_delay:      p[55],
        lfo_pmd:        p[56],
        lfo_amd:        p[57],
        lfo_sync:       p[58],
        lfo_wave:       p[59],
        pitch_mod_sens: p[60],
        amp_mod_sens:   p[61],
        transpose:      p[62],
        poly_mono:      p[63],
        pb_range:       p[64],
        porta_mode:     p[65],
        porta_time:     p[66],
        fc_volume:      p[67],
        sustain:        p[68],
        portamento:     p[69],
        chorus:         p[70],
        mw_pitch:       p[71],
        mw_amplitude:   p[72],
        bc_pitch:       p[73],
        bc_amplitude:   p[74],
        bc_pitch_bias:  p[75],
        bc_eg_bias:     p[76],
        name:           p[77..87].try_into().unwrap(),
        pitch_eg_rate:  [p[87], p[88], p[89]],
        pitch_eg_level: [p[90], p[91], p[92]],
    }
}

fn build_op(op: &Dx100Operator) -> [u8; 13] {
    [
        op.ar, op.d1r, op.d2r, op.rr, op.d1l,
        op.kbd_lev_scl, op.kbd_rate_scl, op.eg_bias_sens,
        op.amp_mod_en, op.key_vel_sens, op.out_level,
        op.freq_ratio, op.detune,
    ]
}

fn build_dx100_1voice(v: &Dx100Voice) -> Vec<u8> {
    // ops[3]=OP4, ops[1]=OP2, ops[2]=OP3, ops[0]=OP1
    let mut p = Vec::with_capacity(93);
    p.extend_from_slice(&build_op(&v.ops[3])); // OP4
    p.extend_from_slice(&build_op(&v.ops[1])); // OP2
    p.extend_from_slice(&build_op(&v.ops[2])); // OP3
    p.extend_from_slice(&build_op(&v.ops[0])); // OP1
    p.extend_from_slice(&[
        v.algorithm, v.feedback,
        v.lfo_speed, v.lfo_delay, v.lfo_pmd, v.lfo_amd,
        v.lfo_sync, v.lfo_wave,
        v.pitch_mod_sens, v.amp_mod_sens,
        v.transpose, v.poly_mono, v.pb_range,
        v.porta_mode, v.porta_time,
        v.fc_volume, v.sustain, v.portamento, v.chorus,
        v.mw_pitch, v.mw_amplitude,
        v.bc_pitch, v.bc_amplitude, v.bc_pitch_bias, v.bc_eg_bias,
    ]);
    p.extend_from_slice(&v.name);
    p.extend_from_slice(&v.pitch_eg_rate);
    p.extend_from_slice(&v.pitch_eg_level);
    p
}
