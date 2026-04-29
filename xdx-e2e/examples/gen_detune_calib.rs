/// Generate a SysEx bank for detune coefficient calibration.
///
/// Each voice has two pure carriers at ratio 1.0 with a known detune step difference.
/// No FM modulation, no LFO, no feedback — just two sine-like oscillators beating against each other.
/// The beat frequency reveals the exact per-step detune magnitude.
///
/// Voices 1–6: step offsets ±1/±2/±3 at A4 (note 69, transpose=24 → 440 Hz)
/// Voices 7–9: step offsets +1/+2/+3 at A3 (transpose=12 → 220 Hz)
///             If beat Hz doubles at A4 vs A3 → cents-based; stays same → Hz-based.
///
/// Usage:
///   cargo run -p xdx-e2e --example gen_detune_calib
///   → testdata/syx/detune_calib.syx
///
/// Then record 30-second notes for enough beat cycles:
///   cargo run -p xdx-compare --bin record-eg-bank --release --
///     testdata/syx/detune_calib.syx --midi-out "UM-ONE" --audio-in "<device>"
///     --note 69 --hold 30.0 --release 0.5 --out testdata/wav/detune_calib
///
///   cargo run -p xdx-e2e --example analyze_detune_calib -- testdata/wav/detune_calib
use xdx_core::dx100::{Dx100Operator, Dx100Voice};
use xdx_core::sysex::dx100_encode_32voice;

fn carrier_op(detune: u8) -> Dx100Operator {
    Dx100Operator {
        out_level: 90,
        detune,
        ..Dx100Operator::default() // ar=31 (instant), d1l=15 (full sustain), d2r=0 — all from default
    }
}

fn main() {
    let out_path = "testdata/syx/detune_calib.syx";

    // (step_offset, transpose, carrier_hz_label)
    // step_offset: steps relative to center detune (3)
    // A4=440Hz at MIDI note 69 needs transpose=24 (center)
    // A3=220Hz at MIDI note 69 needs transpose=12
    let configs: &[(i8, u8, u32)] = &[
        (1, 24, 440),
        (2, 24, 440),
        (3, 24, 440),
        (-1, 24, 440),
        (-2, 24, 440),
        (-3, 24, 440),
        (1, 12, 220),
        (2, 12, 220),
        (3, 12, 220),
    ];

    let mut voices = vec![Dx100Voice::default(); 32];

    for (vi, &(step, transpose, carrier_hz)) in configs.iter().enumerate() {
        let det_center = 3u8;
        let det_offset = (3i8 + step).clamp(0, 6) as u8;

        let sign = if step >= 0 { '+' } else { '-' };
        let label = format!("{}{} {}Hz", sign, step.abs(), carrier_hz);
        let mut name = [b' '; 10];
        for (i, b) in label.bytes().take(10).enumerate() {
            name[i] = b;
        }

        voices[vi] = Dx100Voice {
            algorithm: 7, // Algo 8: all 4 operators as independent carriers
            feedback: 0,
            transpose,
            lfo_pmd: 0,
            lfo_amd: 0,
            ops: [
                carrier_op(det_center),   // OP1: center (reference)
                carrier_op(det_offset),   // OP2: offset by step
                Dx100Operator::default(), // OP3: silent
                Dx100Operator::default(), // OP4: silent (no feedback)
            ],
            name,
            ..Dx100Voice::default()
        };
    }

    let syx = dx100_encode_32voice(&voices, 0);
    std::fs::write(out_path, &syx).unwrap_or_else(|e| panic!("write failed: {e}"));
    println!("Written {out_path} ({} voices)", configs.len());
    println!();
    for (vi, &(step, _, hz)) in configs.iter().enumerate() {
        let sign = if step >= 0 { '+' } else { '-' };
        println!(
            "  Voice {:2}: {}{} step  carrier {}Hz",
            vi + 1,
            sign,
            step.abs(),
            hz
        );
    }
}
