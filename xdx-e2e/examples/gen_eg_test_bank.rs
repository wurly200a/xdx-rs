/// Generate a 24-voice EG calibration bank for DX100 comparison.
///
/// All voices use Algorithm 0 with OP1 as the sole carrier (pure sine, no modulation).
/// OP2/3/4 are silenced (out_level=0). All ops share the same EG parameters so the
/// displayed values match what OP1 actually uses.
///
/// Groups:
///   A: AR sweep  (1-6)   – D1R=10, D1L= 0, D2R= 0, RR= 7
///   B: D1R sweep (7-10)  – AR=25,  D1L= 8, D2R= 0, RR= 7
///   C: D1L sweep (11-14) – AR=25,  D1R=10, D2R= 0, RR= 7
///   D: D2R sweep (15-18) – AR=25,  D1R=15, D1L= 8, RR= 7
///   E: RR sweep  (19-21) – AR=25,  D1R=10, D1L= 8, D2R= 5
///   F: Presets   (22-24) – Piano / Organ / Pluck
///
/// Usage:
///   cargo run -p xdx-e2e --example gen_eg_test_bank
use xdx_core::dx100::{Dx100Operator, Dx100Voice, BANK_VOICES};
use xdx_core::sysex::dx100_encode_32voice;

struct Eg {
    ar: u8,
    d1r: u8,
    d1l: u8,
    d2r: u8,
    rr: u8,
    name: &'static str,
}

fn make_voice(p: &Eg) -> Dx100Voice {
    let op = Dx100Operator {
        ar: p.ar,
        d1r: p.d1r,
        d2r: p.d2r,
        rr: p.rr,
        d1l: p.d1l,
        out_level: 0,
        freq_ratio: 4, // ×1.0
        detune: 3,     // center
        kbd_lev_scl: 0,
        kbd_rate_scl: 0,
        eg_bias_sens: 0,
        amp_mod_en: 0,
        key_vel_sens: 0,
    };
    let carrier = Dx100Operator {
        out_level: 99,
        ..op
    };

    let mut name = [b' '; 10];
    for (i, b) in p.name.as_bytes().iter().take(10).enumerate() {
        name[i] = *b;
    }

    Dx100Voice {
        ops: [carrier, op.clone(), op.clone(), op], // ops[0]=OP1 audible, ops[1-3] silent
        algorithm: 0,
        feedback: 0,
        transpose: 24,
        name,
        ..Default::default()
    }
}

fn main() {
    #[rustfmt::skip]
    let params: Vec<Eg> = vec![
        // ── Group A: AR sweep ── D1R=10, D1L=0, D2R=0, RR=7
        Eg { ar:  5, d1r: 10, d1l:  0, d2r:  0, rr:  7, name: "AR05" },
        Eg { ar: 10, d1r: 10, d1l:  0, d2r:  0, rr:  7, name: "AR10" },
        Eg { ar: 15, d1r: 10, d1l:  0, d2r:  0, rr:  7, name: "AR15" },
        Eg { ar: 20, d1r: 10, d1l:  0, d2r:  0, rr:  7, name: "AR20" },
        Eg { ar: 25, d1r: 10, d1l:  0, d2r:  0, rr:  7, name: "AR25" },
        Eg { ar: 31, d1r: 10, d1l:  0, d2r:  0, rr:  7, name: "AR31" },
        // ── Group B: D1R sweep ── AR=25, D1L=8, D2R=0, RR=7
        Eg { ar: 25, d1r:  5, d1l:  8, d2r:  0, rr:  7, name: "D1R05" },
        Eg { ar: 25, d1r: 10, d1l:  8, d2r:  0, rr:  7, name: "D1R10" },
        Eg { ar: 25, d1r: 15, d1l:  8, d2r:  0, rr:  7, name: "D1R15" },
        Eg { ar: 25, d1r: 20, d1l:  8, d2r:  0, rr:  7, name: "D1R20" },
        // ── Group C: D1L sweep ── AR=25, D1R=10, D2R=0, RR=7
        Eg { ar: 25, d1r: 10, d1l:  0, d2r:  0, rr:  7, name: "D1L00" },
        Eg { ar: 25, d1r: 10, d1l:  5, d2r:  0, rr:  7, name: "D1L05" },
        Eg { ar: 25, d1r: 10, d1l: 10, d2r:  0, rr:  7, name: "D1L10" },
        Eg { ar: 25, d1r: 10, d1l: 14, d2r:  0, rr:  7, name: "D1L14" },
        // ── Group D: D2R sweep ── AR=25, D1R=15, D1L=8, RR=7
        Eg { ar: 25, d1r: 15, d1l:  8, d2r:  5, rr:  7, name: "D2R05" },
        Eg { ar: 25, d1r: 15, d1l:  8, d2r: 10, rr:  7, name: "D2R10" },
        Eg { ar: 25, d1r: 15, d1l:  8, d2r: 15, rr:  7, name: "D2R15" },
        Eg { ar: 25, d1r: 15, d1l:  8, d2r: 25, rr:  7, name: "D2R25" },
        // ── Group E: RR sweep ── AR=25, D1R=10, D1L=8, D2R=5
        Eg { ar: 25, d1r: 10, d1l:  8, d2r:  5, rr:  3, name: "RR03" },
        Eg { ar: 25, d1r: 10, d1l:  8, d2r:  5, rr:  7, name: "RR07" },
        Eg { ar: 25, d1r: 10, d1l:  8, d2r:  5, rr: 13, name: "RR13" },
        // ── Group F: representative shapes
        Eg { ar: 25, d1r: 20, d1l:  0, d2r:  0, rr: 10, name: "PIANO" },
        Eg { ar: 31, d1r:  0, d1l: 15, d2r:  0, rr: 10, name: "ORGAN" },
        Eg { ar: 25, d1r: 15, d1l:  0, d2r:  0, rr: 15, name: "PLUCK" },
    ];

    assert_eq!(
        params.len(),
        BANK_VOICES,
        "must be exactly {BANK_VOICES} voices"
    );

    let voices: Vec<Dx100Voice> = params.iter().map(make_voice).collect();

    println!(
        "{:<3}  {:<10}  {:>3}  {:>3}  {:>3}  {:>3}  {:>3}",
        "#", "Name", "AR", "D1R", "D1L", "D2R", "RR"
    );
    println!("{}", "-".repeat(40));
    for (i, (v, p)) in voices.iter().zip(params.iter()).enumerate() {
        let op = &v.ops[0];
        println!(
            "{:<3}  {:<10}  {:>3}  {:>3}  {:>3}  {:>3}  {:>3}",
            i + 1,
            p.name,
            op.ar,
            op.d1r,
            op.d1l,
            op.d2r,
            op.rr
        );
    }

    let out_path = "testdata/syx/eg_test_bank.syx";
    let syx = dx100_encode_32voice(&voices, 0);
    std::fs::write(out_path, &syx).expect("write failed");
    println!("\nWrote {out_path}  ({} bytes)", syx.len());
}
