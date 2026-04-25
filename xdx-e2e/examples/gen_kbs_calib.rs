/// Generate an 8-voice calibration bank for kbd_lev_scl / kbd_rate_scl tuning.
///
/// All voices: Algorithm 0, OP1 as sole carrier (pure sine), OP2/3/4 silent.
/// Record this bank at multiple MIDI notes to observe how each parameter changes:
///
///   MIDI 48 (C3 std):  -12 st from breakpoint → no kls/krs effect expected
///   MIDI 60 (C4 std):    0 st from breakpoint → baseline (no effect)
///   MIDI 72 (C5 std):  +12 st from breakpoint → moderate effect
///   MIDI 84 (C6 std):  +24 st from breakpoint → strong effect
///
/// Voice layout:
///   Group A – kbd_lev_scl sweep (sustained tone, amplitude shows kls effect):
///     1: SUST_BASE  kls= 0  krs=0  D1R=0  (reference)
///     2: KLS_025    kls=25  krs=0  D1R=0
///     3: KLS_050    kls=50  krs=0  D1R=0
///     4: KLS_099    kls=99  krs=0  D1R=0
///
///   Group B – kbd_rate_scl sweep (decaying tone, decay speed shows krs effect):
///     5: DCY_BASE   kls= 0  krs=0  D1R=10  (reference)
///     6: KRS1_D10   kls= 0  krs=1  D1R=10
///     7: KRS2_D10   kls= 0  krs=2  D1R=10
///     8: KRS3_D10   kls= 0  krs=3  D1R=10
///
/// Usage:
///   cargo run -p xdx-e2e --example gen_kbs_calib
///
/// Recording (repeat for each note):
///   cargo run -p xdx-compare --bin record-eg-bank --release -- \
///     testdata/syx/kbs_calib.syx \
///     --midi-out "UM-ONE" --audio-in "<device>" \
///     --note 60 --hold 2.0 --release 0.5 \
///     --out out/kbs_calib/n60
use xdx_core::dx100::{Dx100Operator, Dx100Voice};
use xdx_core::sysex::dx100_encode_32voice;

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
        freq_ratio: 4,
        detune: 3,
    }
}

fn make_voice(name: &[u8; 10], carrier: Dx100Operator) -> Dx100Voice {
    Dx100Voice {
        ops: [carrier, silent_op(), silent_op(), silent_op()],
        algorithm: 0,
        feedback: 0,
        lfo_speed: 0,
        lfo_delay: 0,
        lfo_pmd: 0,
        lfo_amd: 0,
        lfo_sync: 0,
        lfo_wave: 0,
        pitch_mod_sens: 0,
        amp_mod_sens: 0,
        transpose: 24,
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
    // ── Group A: kbd_lev_scl sweep ─────────────────────────────────────────────
    // Sustained tone (D1R=0 → holds at peak amplitude throughout hold).
    // Measure the steady-state amplitude at each note to derive kls formula.
    let group_a: &[(&[u8; 10], u8)] = &[
        (b"SUST_BASE ", 0),
        (b"KLS_025   ", 25),
        (b"KLS_050   ", 50),
        (b"KLS_099   ", 99),
    ];

    // ── Group B: kbd_rate_scl sweep ────────────────────────────────────────────
    // Decaying tone (D1R=10, decays from peak to silence over ~1.5s at note 60).
    // Measure how quickly the envelope decays at each note to derive krs formula.
    let group_b: &[(&[u8; 10], u8)] = &[
        (b"DCY_BASE  ", 0),
        (b"KRS1_D10  ", 1),
        (b"KRS2_D10  ", 2),
        (b"KRS3_D10  ", 3),
    ];

    let mut voices: Vec<Dx100Voice> = Vec::new();

    for &(name, kls) in group_a {
        voices.push(make_voice(
            name,
            Dx100Operator {
                ar: 31,
                d1r: 0,
                d1l: 15,
                d2r: 0,
                rr: 15,
                kbd_lev_scl: kls,
                kbd_rate_scl: 0,
                out_level: 90,
                ..silent_op()
            },
        ));
    }

    for &(name, krs) in group_b {
        voices.push(make_voice(
            name,
            Dx100Operator {
                ar: 31,
                d1r: 10,
                d1l: 0,
                d2r: 0,
                rr: 15,
                kbd_lev_scl: 0,
                kbd_rate_scl: krs,
                out_level: 90,
                ..silent_op()
            },
        ));
    }

    assert_eq!(voices.len(), 8, "expected 8 calibration voices");

    // Pad to 32 with defaults
    while voices.len() < 32 {
        voices.push(Dx100Voice::default());
    }

    let syx = dx100_encode_32voice(&voices, 0);
    let path = "testdata/syx/kbs_calib.syx";
    std::fs::write(path, &syx).expect("write syx");

    println!("Written {} bytes → {path}", syx.len());
    println!();
    println!("Voices:");
    for (i, v) in voices.iter().take(8).enumerate() {
        let op = &v.ops[0];
        println!(
            "  {:2}: {:<10}  kls={:2}  krs={}  AR={:2} D1R={:2} D1L={:2}",
            i + 1,
            v.name_str(),
            op.kbd_lev_scl,
            op.kbd_rate_scl,
            op.ar,
            op.d1r,
            op.d1l,
        );
    }
    println!();
    println!("Record at 4 notes (adjust --midi-out / --audio-in as needed):");
    for note in [48u8, 60, 72, 84] {
        let note_name = ["C3", "C4", "C5", "C6"][(note / 12 - 4) as usize];
        let offset = note as i32 - 60;
        println!(
            "  cargo run -p xdx-compare --bin record-eg-bank --release -- \
            testdata/syx/kbs_calib.syx \\\n    \
            --midi-out \"UM-ONE\" --audio-in \"<device>\" \\\n    \
            --note {note} --hold 2.0 --release 0.5 --out out/kbs_calib/n{note}  \
            # {note_name} ({offset:+}st)"
        );
    }
}
