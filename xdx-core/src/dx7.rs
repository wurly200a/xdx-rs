/// DX7 operator parameters (6 operators per voice)
#[derive(Debug, Clone, PartialEq)]
pub struct Dx7Operator {
    pub eg_rate: [u8; 4],        // EG Rate 1-4 (0-99)
    pub eg_level: [u8; 4],       // EG Level 1-4 (0-99)
    pub kbd_lev_scl_brk_pt: u8,  // Keyboard Level Scaling Break Point (0-99)
    pub kbd_lev_scl_lft_dep: u8, // Left Depth (0-99)
    pub kbd_lev_scl_rht_dep: u8, // Right Depth (0-99)
    pub kbd_lev_scl_lft_crv: u8, // Left Curve (0-3)
    pub kbd_lev_scl_rht_crv: u8, // Right Curve (0-3)
    pub kbd_rate_scaling: u8,    // (0-7)
    pub amp_mod_sensitivity: u8, // (0-3)
    pub key_vel_sensitivity: u8, // (0-7)
    pub output_level: u8,        // (0-99)
    pub osc_mode: u8,            // Oscillator Mode: 0=ratio, 1=fixed (0-1)
    pub osc_freq_coarse: u8,     // (0-31)
    pub osc_freq_fine: u8,       // (0-99)
    pub osc_detune: u8,          // (0-14, center=7)
}

/// DX7 voice parameters (1 voice = 6 operators + global params)
#[derive(Debug, Clone, PartialEq)]
pub struct Dx7Voice {
    pub operators: [Dx7Operator; 6], // OP6..OP1 (SysEx order)
    pub pitch_eg_rate: [u8; 4],      // Pitch EG Rate 1-4 (0-99)
    pub pitch_eg_level: [u8; 4],     // Pitch EG Level 1-4 (0-99)
    pub algorithm: u8,               // Algorithm (0-31, display 1-32)
    pub feedback: u8,                // (0-7)
    pub osc_key_sync: u8,            // (0-1)
    pub lfo_speed: u8,               // (0-99)
    pub lfo_delay: u8,               // (0-99)
    pub lfo_pitch_mod_dep: u8,       // (0-99)
    pub lfo_amp_mod_dep: u8,         // (0-99)
    pub lfo_sync: u8,                // (0-1)
    pub lfo_wave: u8,                // (0-5)
    pub pitch_mod_sensitivity: u8,   // (0-7)
    pub transpose: u8,               // (0-48, center=24)
    pub name: [u8; 10],              // Voice name (ASCII)
}

impl Dx7Voice {
    pub fn name_str(&self) -> String {
        self.name
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '?'
                }
            })
            .collect::<String>()
            .trim_end()
            .to_string()
    }
}
