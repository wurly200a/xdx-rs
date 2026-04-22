/// Fine-grained WAV analysis: find EG discontinuities.
/// Prints 1ms RMS from 0-1500ms, and also scans for the largest
/// sample-to-sample jump in the entire file.
use xdx_core::sysex::dx100_decode_1voice;

fn main() {
    // ── Print IvoryEbony params ───────────────────────────────────────────────
    let bytes = std::fs::read("testdata/syx/IvoryEbony.syx").expect("read IvoryEbony.syx");
    let voice = dx100_decode_1voice(&bytes).expect("decode");
    println!("=== IvoryEbony parameters ===");
    println!(
        "Algorithm={} Feedback={}",
        voice.algorithm + 1,
        voice.feedback
    );
    for (i, op) in voice.ops.iter().enumerate() {
        println!(
            "  OP{}: AR={:2} D1R={:2} D2R={:2} RR={:2} D1L={:2}  Level={:2}  Ratio={}  Det={}",
            i + 1,
            op.ar,
            op.d1r,
            op.d2r,
            op.rr,
            op.d1l,
            op.out_level,
            op.freq_ratio,
            op.detune
        );
    }
    println!();

    // ── Load WAV ─────────────────────────────────────────────────────────────
    let wav_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "out/ivory_analyze/synth.wav".to_string());
    let mut reader = hound::WavReader::open(&wav_path).expect("open wav");
    let sr = reader.spec().sample_rate as f32;
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();

    // ── 1. Scan for largest sample-to-sample jump ────────────────────────────
    let mut max_jump = 0.0f32;
    let mut max_jump_sample = 0usize;
    for i in 1..samples.len() {
        let j = (samples[i] - samples[i - 1]).abs();
        if j > max_jump {
            max_jump = j;
            max_jump_sample = i;
        }
    }
    println!("=== Largest sample jump ===");
    println!(
        "  jump={:.5}  at sample {}  ({:.2}ms)",
        max_jump,
        max_jump_sample,
        max_jump_sample as f32 / sr * 1000.0
    );

    // ── 2. Find top-10 largest jumps ─────────────────────────────────────────
    let mut jumps: Vec<(f32, usize)> = (1..samples.len())
        .map(|i| ((samples[i] - samples[i - 1]).abs(), i))
        .collect();
    jumps.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    println!("\n=== Top 10 sample jumps ===");
    for (j, idx) in jumps.iter().take(10) {
        println!(
            "  {:.5}  at {:.2}ms  (sample {})",
            j,
            *idx as f32 / sr * 1000.0,
            idx
        );
    }

    // ── 3. 1ms-window RMS for first 1500ms ───────────────────────────────────
    let window_1ms = (sr / 1000.0) as usize;
    let limit_1500ms = (1500.0 / 1000.0 * sr) as usize;

    println!("\n=== 1ms RMS, 0-1500ms (only changes > 0.01) ===");
    println!("{:>8}  {:>8}  {:>8}", "time_ms", "rms", "delta");
    println!("{}", "-".repeat(36));

    let mut prev_rms = 0.0f32;
    let mut printed_last = false;
    for wi in 0..(limit_1500ms / window_1ms) {
        let start = wi * window_1ms;
        let end = (start + window_1ms).min(samples.len());
        let chunk = &samples[start..end];
        let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
        let delta = rms - prev_rms;
        let t_ms = wi as f32;

        if delta.abs() > 0.01 || (wi % 50 == 0) {
            let mark = if delta.abs() > 0.03 { " ***" } else { "" };
            println!("{:>8.1}  {:>8.5}  {:>+8.5}{}", t_ms, rms, delta, mark);
            printed_last = true;
        } else {
            printed_last = false;
        }
        prev_rms = rms;
    }
    if !printed_last {
        // print last row regardless
        let wi = limit_1500ms / window_1ms - 1;
        let start = wi * window_1ms;
        let end = (start + window_1ms).min(samples.len());
        let chunk = &samples[start..end];
        let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
        let delta = rms - prev_rms;
        println!("{:>8.1}  {:>8.5}  {:>+8.5}", wi as f32, rms, delta);
    }
}
