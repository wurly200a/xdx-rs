/// Generate a 17-voice calibration bank for LFO parameter tuning.
///
/// All voices: Algorithm 0, OP1 as sole carrier (1.00x ratio, sustained),
/// OP2/3/4 silent.  Record at MIDI note 60 (C4) unless noted.
///
/// ── Group A: lfo_speed (7 voices) ────────────────────────────────────────────
/// Goal: map speed (0-99) → Hz by measuring the vibrato oscillation period.
/// Settings: wave=TRI, PMD=99, PMS=7, AMD=0, SYNC=1, DELAY=0.
/// Measure: pitch-modulation period from audio → f = 1/T Hz.
///   SPD_00  speed= 0  → ~0.063 Hz  (period ~16 s — use 40 s recording)
///   SPD_16  speed=16  → ~2.25 Hz   (period ~0.44 s)
///   SPD_33  speed=33  → ~3.50 Hz   (period ~0.29 s)
///   SPD_50  speed=50  → ~7.99 Hz   (period ~0.13 s)
///   SPD_66  speed=66  → ~9.80 Hz   (period ~0.10 s)
///   SPD_83  speed=83  → ~31.2 Hz   (period ~0.032 s)
///   SPD_99  speed=99  → ~49.3 Hz   (period ~0.020 s)
///
/// ── Group B: pitch-mod depth (4 voices) ──────────────────────────────────────
/// Goal: find the actual cent range for (PMD, PMS) pairs.
/// Settings: wave=TRI, speed=5 (~0.75 Hz, period ~1.3 s), SYNC=1, DELAY=0.
/// Measure: peak pitch deviation above/below nominal note (in cents).
///   PMD50_S3  PMD=50 PMS=3
///   PMD99_S3  PMD=99 PMS=3
///   PMD50_S7  PMD=50 PMS=7
///   PMD99_S7  PMD=99 PMS=7
///
/// ── Group C: amplitude-mod depth (3 voices) ──────────────────────────────────
/// Goal: find the actual dB range for (AMD, AMS) pairs.
/// Settings: wave=TRI, speed=5, SYNC=1, DELAY=0, PMD=0 (no pitch mod).
///           amp_mod_en=1 on carrier; amp_mod_sens set per voice.
/// Measure: RMS peak-to-trough amplitude ratio in dB.
///   AMD99_A1  AMD=99 AMS=1
///   AMD99_A2  AMD=99 AMS=2
///   AMD99_A3  AMD=99 AMS=3
///
/// ── Group D: lfo_delay (3 voices) ─────────────────────────────────────────────
/// Goal: map delay (0-99) → seconds until LFO ramps to full depth.
/// Settings: wave=TRI, speed=33 (~3.5 Hz), PMD=99, PMS=7, SYNC=1.
/// Measure: time from note-on to first visible pitch oscillation onset.
///   DLY_025   delay=25
///   DLY_050   delay=50
///   DLY_075   delay=75
///
/// ── Usage ─────────────────────────────────────────────────────────────────────
///   cargo run -p xdx-e2e --example gen_lfo_calib
///
/// Send the bank to DX100 via xdx-gui (32 VOICES → Open → Send).
///
/// Record Group A (long hold for slow speeds):
///   cargo run -p xdx-compare --bin record-eg-bank --release -- \
///     testdata/syx/lfo_calib.syx \
///     --midi-out "UM-ONE" --audio-in "<device>" \
///     --note 60 --hold 8.0 --release 0.5 --out out/lfo_calib/grp_a
///   (For SPD_00 re-record alone with --hold 40.0)
///
/// Record Groups B/C/D (shorter hold):
///   cargo run -p xdx-compare --bin record-eg-bank --release -- \
///     testdata/syx/lfo_calib.syx \
///     --midi-out "UM-ONE" --audio-in "<device>" \
///     --note 60 --hold 5.0 --release 0.5 --out out/lfo_calib/grp_bcd
use xdx_core::dx100::{Dx100Operator, Dx100Voice};
use xdx_core::sysex::dx100_encode_32voice;

/// Silent filler operator (out_level=0).
fn silent_op() -> Dx100Operator {
    Dx100Operator {
        ar: 31,
        d1r: 0,
        d2r: 0,
        rr: 15,
        d1l: 15,
        kbd_lev_scl: 0,
        kbd_rate_scl: 0,
        eg_bias_sens: 0,
        amp_mod_en: 0,
        key_vel_sens: 0,
        out_level: 0,
        freq_ratio: 4, // 1.00x
        detune: 3,     // centre
    }
}

/// Sustained carrier: AR=31 D1R=0 D1L=15 RR=15, out_level=80.
fn carrier(amp_mod_en: u8) -> Dx100Operator {
    Dx100Operator {
        ar: 31,
        d1r: 0,
        d2r: 0,
        rr: 15,
        d1l: 15,
        kbd_lev_scl: 0,
        kbd_rate_scl: 0,
        eg_bias_sens: 0,
        amp_mod_en,
        key_vel_sens: 0,
        out_level: 80,
        freq_ratio: 4, // 1.00x
        detune: 3,     // centre
    }
}

fn make_voice(
    name: &[u8; 10],
    lfo_speed: u8,
    lfo_delay: u8,
    lfo_wave: u8,
    lfo_sync: u8,
    lfo_pmd: u8,
    lfo_amd: u8,
    pitch_mod_sens: u8,
    amp_mod_sens: u8,
) -> Dx100Voice {
    Dx100Voice {
        ops: [
            carrier(if amp_mod_sens > 0 { 1 } else { 0 }),
            silent_op(),
            silent_op(),
            silent_op(),
        ],
        algorithm: 0,
        feedback: 0,
        lfo_speed,
        lfo_delay,
        lfo_pmd,
        lfo_amd,
        lfo_sync,
        lfo_wave,
        pitch_mod_sens,
        amp_mod_sens,
        transpose: 24, // no transpose
        poly_mono: 0,
        pb_range: 0,
        porta_mode: 0,
        porta_time: 0,
        fc_volume: 0,
        sustain: 0,
        portamento: 0,
        chorus: 0,
        mw_pitch: 0,
        mw_amplitude: 0,
        bc_pitch: 0,
        bc_amplitude: 0,
        bc_pitch_bias: 0,
        bc_eg_bias: 0,
        name: *name,
        pitch_eg_rate: [0, 0, 0],
        pitch_eg_level: [50, 50, 50],
    }
}

fn main() {
    let mut voices: Vec<Dx100Voice> = Vec::new();

    // ── Group A: lfo_speed sweep ──────────────────────────────────────────────
    // wave=TRI(2), PMD=99, PMS=7, AMD=0, SYNC=1, DELAY=0
    let speed_voices: &[(&[u8; 10], u8)] = &[
        (b"SPD_00    ", 0),
        (b"SPD_16    ", 16),
        (b"SPD_33    ", 33),
        (b"SPD_50    ", 50),
        (b"SPD_66    ", 66),
        (b"SPD_83    ", 83),
        (b"SPD_99    ", 99),
    ];
    for &(name, speed) in speed_voices {
        voices.push(make_voice(name, speed, 0, 2, 1, 99, 0, 7, 0));
    }

    // ── Group B: pitch-mod depth ──────────────────────────────────────────────
    // wave=TRI(2), speed=5, SYNC=1, DELAY=0, AMD=0
    let pmd_voices: &[(&[u8; 10], u8, u8)] = &[
        (b"PMD50_S3  ", 50, 3),
        (b"PMD99_S3  ", 99, 3),
        (b"PMD50_S7  ", 50, 7),
        (b"PMD99_S7  ", 99, 7),
    ];
    for &(name, pmd, pms) in pmd_voices {
        voices.push(make_voice(name, 5, 0, 2, 1, pmd, 0, pms, 0));
    }

    // ── Group C: amplitude-mod depth ─────────────────────────────────────────
    // wave=TRI(2), speed=5, SYNC=1, DELAY=0, PMD=0, AMD=99
    // amp_mod_en on carrier is set inside make_voice when amp_mod_sens > 0
    let amd_voices: &[(&[u8; 10], u8)] =
        &[(b"AMD99_A1  ", 1), (b"AMD99_A2  ", 2), (b"AMD99_A3  ", 3)];
    for &(name, ams) in amd_voices {
        voices.push(make_voice(name, 5, 0, 2, 1, 0, 99, 0, ams));
    }

    // ── Group D: lfo_delay sweep ──────────────────────────────────────────────
    // wave=TRI(2), speed=33, PMD=99, PMS=7, SYNC=1
    let delay_voices: &[(&[u8; 10], u8)] = &[
        (b"DLY_025   ", 25),
        (b"DLY_050   ", 50),
        (b"DLY_075   ", 75),
    ];
    for &(name, delay) in delay_voices {
        voices.push(make_voice(name, 33, delay, 2, 1, 99, 0, 7, 0));
    }

    assert_eq!(voices.len(), 17);

    // Pad to 32 with defaults
    while voices.len() < 32 {
        voices.push(Dx100Voice::default());
    }

    let syx = dx100_encode_32voice(&voices, 0);
    let path = "testdata/syx/lfo_calib.syx";
    std::fs::write(path, &syx).expect("write syx");

    println!("Written {} bytes → {path}", syx.len());
    println!();
    println!("── Group A: lfo_speed sweep (wave=TRI PMD=99 PMS=7) ──");
    for v in voices.iter().take(7) {
        println!("  {:<10}  speed={:2}", v.name_str(), v.lfo_speed);
    }
    println!();
    println!("── Group B: pitch-mod depth (wave=TRI speed=5) ──");
    for v in voices.iter().skip(7).take(4) {
        println!(
            "  {:<10}  PMD={:2}  PMS={}",
            v.name_str(),
            v.lfo_pmd,
            v.pitch_mod_sens
        );
    }
    println!();
    println!("── Group C: amp-mod depth (wave=TRI speed=5 AMD=99) ──");
    for v in voices.iter().skip(11).take(3) {
        println!(
            "  {:<10}  AMD={:2}  AMS={}",
            v.name_str(),
            v.lfo_amd,
            v.amp_mod_sens
        );
    }
    println!();
    println!("── Group D: lfo_delay sweep (wave=TRI speed=33 PMD=99 PMS=7) ──");
    for v in voices.iter().skip(14).take(3) {
        println!("  {:<10}  delay={:2}", v.name_str(), v.lfo_delay);
    }
    println!();
    println!("Send to DX100 via xdx-gui (32 VOICES → Open lfo_calib.syx → Send).");
    println!();
    println!("Record Groups A (hold=8s) — re-record SPD_00 separately with hold=40s:");
    println!("  cargo run -p xdx-compare --bin record-eg-bank --release -- \\");
    println!("    testdata/syx/lfo_calib.syx \\");
    println!("    --midi-out \"UM-ONE\" --audio-in \"<device>\" \\");
    println!("    --note 60 --hold 8.0 --release 0.5 --out out/lfo_calib/grp_a");
    println!();
    println!("Record Groups B/C/D (hold=5s):");
    println!("  cargo run -p xdx-compare --bin record-eg-bank --release -- \\");
    println!("    testdata/syx/lfo_calib.syx \\");
    println!("    --midi-out \"UM-ONE\" --audio-in \"<device>\" \\");
    println!("    --note 60 --hold 5.0 --release 0.5 --out out/lfo_calib/grp_bcd");
}
