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

// ── DX100 32-voice bulk dump (4104 bytes total) ───────────────────────────────
// F0 43 0n 04 20 00 <4096 bytes payload> <checksum> F7
//
// Payload: 32 × 128-byte VMEM blocks (only first 73 bytes are real data)
//
// VMEM layout per voice (128 bytes, significant bytes 0-72):
//   [00-05]  OP4: ar, d1r, d2r, rr, d1l, kbd_lev_scl
//   [06]     OP4: (amp_mod_en<<6)|(eg_bias_sens<<3)|key_vel_sens
//   [07]     OP4: out_level
//   [08]     OP4: freq_ratio
//   [09]     OP4: (kbd_rate_scl<<3)|detune
//   [10-15]  OP2: ar, d1r, d2r, rr, d1l, kbd_lev_scl
//   [16]     OP2: (amp_mod_en<<6)|(eg_bias_sens<<3)|key_vel_sens
//   [17]     OP2: out_level
//   [18]     OP2: freq_ratio
//   [19]     OP2: (kbd_rate_scl<<3)|detune
//   [20-25]  OP3: ar, d1r, d2r, rr, d1l, kbd_lev_scl
//   [26]     OP3: (amp_mod_en<<6)|(eg_bias_sens<<3)|key_vel_sens
//   [27]     OP3: out_level
//   [28]     OP3: freq_ratio
//   [29]     OP3: (kbd_rate_scl<<3)|detune
//   [30-35]  OP1: ar, d1r, d2r, rr, d1l, kbd_lev_scl
//   [36]     OP1: (amp_mod_en<<6)|(eg_bias_sens<<3)|key_vel_sens
//   [37]     OP1: out_level
//   [38]     OP1: freq_ratio
//   [39]     OP1: (kbd_rate_scl<<3)|detune
//   [40]     (lfo_sync<<6)|(feedback<<3)|algorithm
//   [41]     lfo_speed
//   [42]     lfo_delay
//   [43]     lfo_pmd
//   [44]     lfo_amd
//   [45]     (pitch_mod_sens<<4)|(amp_mod_sens<<2)|lfo_wave
//   [46]     transpose
//   [47]     pb_range
//   [48]     (chorus<<4)|(poly_mono<<3)|(sustain<<2)|(portamento<<1)|porta_mode
//   [49]     porta_time
//   [50]     fc_volume
//   [51]     mw_pitch
//   [52]     mw_amplitude
//   [53]     bc_pitch
//   [54]     bc_amplitude
//   [55]     bc_pitch_bias
//   [56]     bc_eg_bias
//   [57-66]  name (10 ASCII bytes)
//   [67-69]  pitch_eg_rate[0..2]
//   [70-72]  pitch_eg_level[0..2]
//   [73-127] dummy/reserved (zeros)

const DX100_32VOICE_PAYLOAD_LEN: u16 = 4096; // 0x2000
const DX100_32VOICE_TOTAL_LEN:   usize = 4104;
const DX100_VMEM_SIZE:           usize = 128;

pub fn dx100_decode_32voice(data: &[u8]) -> Result<Vec<Dx100Voice>, SysExError> {
    if data.len() < DX100_32VOICE_TOTAL_LEN {
        return Err(SysExError::TooShort);
    }
    if data[0] != 0xF0 || data[1] != 0x43 || data[3] != 0x04 {
        return Err(SysExError::InvalidHeader);
    }
    if data[DX100_32VOICE_TOTAL_LEN - 1] != 0xF7 {
        return Err(SysExError::InvalidFooter);
    }
    let byte_count = ((data[4] as u16) << 7) | (data[5] as u16);
    if byte_count != DX100_32VOICE_PAYLOAD_LEN {
        return Err(SysExError::InvalidByteCount {
            expected: DX100_32VOICE_PAYLOAD_LEN,
            actual: byte_count,
        });
    }
    let payload = &data[6..6 + DX100_32VOICE_PAYLOAD_LEN as usize];
    verify_checksum(payload, data[6 + DX100_32VOICE_PAYLOAD_LEN as usize])?;

    let voices = (0..32)
        .map(|i| vmem_to_voice(&payload[i * DX100_VMEM_SIZE..]))
        .collect();
    Ok(voices)
}

pub fn dx100_encode_32voice(voices: &[Dx100Voice], channel: u8) -> Vec<u8> {
    let mut payload = Vec::with_capacity(DX100_32VOICE_PAYLOAD_LEN as usize);
    for i in 0..32 {
        let vmem = voice_to_vmem(voices.get(i).unwrap_or(&Dx100Voice::default()));
        payload.extend_from_slice(&vmem);
    }
    let checksum = calc_checksum(&payload);
    let mut out = vec![
        0xF0, 0x43, 0x00 | (channel & 0x0F), 0x04,
        0x20, 0x00,
    ];
    out.extend_from_slice(&payload);
    out.push(checksum);
    out.push(0xF7);
    out
}

fn vmem_to_voice(v: &[u8]) -> Dx100Voice {
    let op = |base: usize| Dx100Operator {
        ar:           v[base],
        d1r:          v[base + 1],
        d2r:          v[base + 2],
        rr:           v[base + 3],
        d1l:          v[base + 4],
        kbd_lev_scl:  v[base + 5],
        amp_mod_en:   (v[base + 6] >> 6) & 0x1,
        eg_bias_sens: (v[base + 6] >> 3) & 0x7,
        key_vel_sens:  v[base + 6]        & 0x7,
        out_level:     v[base + 7],
        freq_ratio:    v[base + 8],
        kbd_rate_scl: (v[base + 9] >> 3) & 0x1F,
        detune:        v[base + 9]        & 0x7,
    };
    Dx100Voice {
        ops: [op(30), op(10), op(20), op(0)], // [OP1, OP2, OP3, OP4]
        algorithm:      v[40] & 0x7,
        feedback:      (v[40] >> 3) & 0x7,
        lfo_sync:      (v[40] >> 6) & 0x3,
        lfo_speed:      v[41],
        lfo_delay:      v[42],
        lfo_pmd:        v[43],
        lfo_amd:        v[44],
        lfo_wave:       v[45] & 0x3,
        amp_mod_sens:  (v[45] >> 2) & 0x3,
        pitch_mod_sens:(v[45] >> 4) & 0x7,
        transpose:      v[46],
        pb_range:       v[47],
        porta_mode:     v[48] & 0x1,
        portamento:    (v[48] >> 1) & 0x1,
        sustain:       (v[48] >> 2) & 0x1,
        poly_mono:     (v[48] >> 3) & 0x1,
        chorus:        (v[48] >> 4) & 0x1,
        porta_time:     v[49],
        fc_volume:      v[50],
        mw_pitch:       v[51],
        mw_amplitude:   v[52],
        bc_pitch:       v[53],
        bc_amplitude:   v[54],
        bc_pitch_bias:  v[55],
        bc_eg_bias:     v[56],
        name:           v[57..67].try_into().unwrap(),
        pitch_eg_rate:  [v[67], v[68], v[69]],
        pitch_eg_level: [v[70], v[71], v[72]],
    }
}

fn voice_to_vmem(v: &Dx100Voice) -> [u8; DX100_VMEM_SIZE] {
    let mut m = [0u8; DX100_VMEM_SIZE];
    // op_base: (vmem_offset, ops_index)  order: OP4=ops[3], OP2=ops[1], OP3=ops[2], OP1=ops[0]
    for (base, idx) in [(0, 3), (10, 1), (20, 2), (30, 0)] {
        let op = &v.ops[idx];
        m[base]     = op.ar;
        m[base + 1] = op.d1r;
        m[base + 2] = op.d2r;
        m[base + 3] = op.rr;
        m[base + 4] = op.d1l;
        m[base + 5] = op.kbd_lev_scl;
        m[base + 6] = ((op.amp_mod_en   & 0x1) << 6)
                    | ((op.eg_bias_sens  & 0x7) << 3)
                    |  (op.key_vel_sens  & 0x7);
        m[base + 7] = op.out_level;
        m[base + 8] = op.freq_ratio;
        m[base + 9] = ((op.kbd_rate_scl & 0x1F) << 3)
                    |  (op.detune        & 0x7);
    }
    m[40] = ((v.lfo_sync  & 0x3) << 6)
          | ((v.feedback  & 0x7) << 3)
          |  (v.algorithm & 0x7);
    m[41] = v.lfo_speed;
    m[42] = v.lfo_delay;
    m[43] = v.lfo_pmd;
    m[44] = v.lfo_amd;
    m[45] = ((v.pitch_mod_sens & 0x7) << 4)
          | ((v.amp_mod_sens   & 0x3) << 2)
          |  (v.lfo_wave       & 0x3);
    m[46] = v.transpose;
    m[47] = v.pb_range;
    m[48] = ((v.chorus     & 0x1) << 4)
          | ((v.poly_mono  & 0x1) << 3)
          | ((v.sustain    & 0x1) << 2)
          | ((v.portamento & 0x1) << 1)
          |  (v.porta_mode & 0x1);
    m[49] = v.porta_time;
    m[50] = v.fc_volume;
    m[51] = v.mw_pitch;
    m[52] = v.mw_amplitude;
    m[53] = v.bc_pitch;
    m[54] = v.bc_amplitude;
    m[55] = v.bc_pitch_bias;
    m[56] = v.bc_eg_bias;
    m[57..67].copy_from_slice(&v.name);
    m[67] = v.pitch_eg_rate[0];
    m[68] = v.pitch_eg_rate[1];
    m[69] = v.pitch_eg_rate[2];
    m[70] = v.pitch_eg_level[0];
    m[71] = v.pitch_eg_level[1];
    m[72] = v.pitch_eg_level[2];
    m
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
