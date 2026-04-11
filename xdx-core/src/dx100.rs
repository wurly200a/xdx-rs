/// DX100 operator parameters
/// SysEx payload order: OP4(0-12), OP2(13-25), OP3(26-38), OP1(39-51)
/// Stored in this struct as ops[0]=OP1 .. ops[3]=OP4
#[derive(Debug, Clone, PartialEq)]
pub struct Dx100Operator {
    pub ar:           u8, // Attack Rate          (0-31)
    pub d1r:          u8, // Decay 1 Rate         (0-31)
    pub d2r:          u8, // Decay 2 Rate         (0-31)
    pub rr:           u8, // Release Rate         (0-15)
    pub d1l:          u8, // Decay 1 Level        (0-15)
    pub kbd_lev_scl:  u8, // Keyboard Level Scaling (0-99)
    pub kbd_rate_scl: u8, // Keyboard Rate Scaling  (0-3)
    pub eg_bias_sens: u8, // EG Bias Sensitivity    (0-7)
    pub amp_mod_en:   u8, // Amplitude Mod Enable   (0-1)
    pub key_vel_sens: u8, // Key Velocity Sensitivity (0-7)
    pub out_level:    u8, // Output Level           (0-99)
    pub freq_ratio:   u8, // Oscillator Frequency   (0-63, ratio mode) or fixed
    pub detune:       u8, // Detune                 (0-6, center=3)
}

/// DX100 voice parameters
/// ops[0]=OP1, ops[1]=OP2, ops[2]=OP3, ops[3]=OP4
#[derive(Debug, Clone, PartialEq)]
pub struct Dx100Voice {
    pub ops:            [Dx100Operator; 4],
    pub algorithm:      u8, // (0-7, display 1-8)
    pub feedback:       u8, // (0-7)
    pub lfo_speed:      u8, // (0-99)
    pub lfo_delay:      u8, // (0-99)
    pub lfo_pmd:        u8, // LFO Pitch Mod Depth  (0-99)
    pub lfo_amd:        u8, // LFO Amp Mod Depth    (0-99)
    pub lfo_sync:       u8, // (0-1)
    pub lfo_wave:       u8, // (0-3)
    pub pitch_mod_sens: u8, // Pitch Mod Sensitivity (0-7)
    pub amp_mod_sens:   u8, // Amp Mod Sensitivity   (0-3)
    pub transpose:      u8, // (0-48, center=24)
    pub poly_mono:      u8, // 0=poly, 1=mono
    pub pb_range:       u8, // Pitch Bend Range       (0-12)
    pub porta_mode:     u8, // Portamento Mode        (0-1)
    pub porta_time:     u8, // Portamento Time        (0-99)
    pub fc_volume:      u8, // Foot Controller Volume (0-99)
    pub sustain:        u8, // Sustain foot switch    (0-1)
    pub portamento:     u8, // Portamento foot switch (0-1)
    pub chorus:         u8, // Chorus switch          (0-1)
    pub mw_pitch:       u8, // Mod Wheel Pitch range  (0-99)
    pub mw_amplitude:   u8, // Mod Wheel Amp range    (0-99)
    pub bc_pitch:       u8, // Breath Ctrl Pitch range     (0-99)
    pub bc_amplitude:   u8, // Breath Ctrl Amp range       (0-99)
    pub bc_pitch_bias:  u8, // Breath Ctrl Pitch Bias range(0-99)
    pub bc_eg_bias:     u8, // Breath Ctrl EG Bias range   (0-99)
    pub name:           [u8; 10],
    pub pitch_eg_rate:  [u8; 3], // PEG Rate 1-3  (0-99)
    pub pitch_eg_level: [u8; 3], // PEG Level 1-3 (0-99)
}

impl Dx100Voice {
    pub fn name_str(&self) -> String {
        self.name.iter()
            .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '?' })
            .collect::<String>()
            .trim_end()
            .to_string()
    }
}
