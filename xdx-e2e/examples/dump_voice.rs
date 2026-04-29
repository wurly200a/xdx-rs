/// Decode a 32-voice bank SysEx and print parameters for one or more voice indices.
/// Usage:
///   cargo run -p xdx-e2e --example dump_voice -- <bank.syx> [index1 index2 ...]
/// Index is 1-based (1 = first voice).
use xdx_core::dx100::FREQ_RATIOS;
use xdx_core::sysex::dx100_decode_32voice;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: dump_voice <bank.syx> [index1 index2 ...]");
        std::process::exit(1);
    }
    let syx_path = &args[0];
    let indices: Vec<usize> = if args.len() > 1 {
        args[1..]
            .iter()
            .map(|s| s.parse::<usize>().expect("index"))
            .collect()
    } else {
        (1..=24).collect()
    };

    let bytes = std::fs::read(syx_path).unwrap_or_else(|e| panic!("Cannot read {syx_path}: {e}"));
    let voices = dx100_decode_32voice(&bytes).unwrap_or_else(|e| panic!("Decode failed: {e:?}"));

    for idx in indices {
        let i = idx - 1;
        if i >= voices.len() {
            eprintln!(
                "Voice {idx} out of range (bank has {} voices)",
                voices.len()
            );
            continue;
        }
        let v = &voices[i];
        println!("=== Voice {idx}: \"{}\" ===", v.name_str());
        println!(
            "  Algo: {}  FB: {}  Transpose: {} (offset {}st)",
            v.algorithm + 1,
            v.feedback,
            v.transpose,
            v.transpose as i8 - 24
        );
        println!(
            "  LFO: speed={} pmd={} amd={} pms={} ams={} wave={} delay={} sync={}",
            v.lfo_speed,
            v.lfo_pmd,
            v.lfo_amd,
            v.pitch_mod_sens,
            v.amp_mod_sens,
            v.lfo_wave,
            v.lfo_delay,
            v.lfo_sync,
        );
        println!(
            "  PEG rate: {:?}  level: {:?}",
            v.pitch_eg_rate, v.pitch_eg_level
        );
        println!("  Operators (OP1=carrier side, OP4=feedback side):");
        for (op_idx, op) in v.ops.iter().enumerate() {
            let ratio = FREQ_RATIOS[op.freq_ratio as usize % 64];
            println!(
                "    OP{}: ratio[{}]={:.3}  det={}  out={}  ar={} d1r={} d1l={} d2r={} rr={}  kls={} krs={}  amp_mod_en={}",
                op_idx + 1,
                op.freq_ratio, ratio,
                op.detune as i8 - 3,
                op.out_level,
                op.ar, op.d1r, op.d1l, op.d2r, op.rr,
                op.kbd_lev_scl, op.kbd_rate_scl,
                op.amp_mod_en,
            );
        }
        println!();
    }
}
