use std::f32::consts::TAU;
use xdx_core::dx100::FREQ_RATIOS;
/// Trace per-operator EG levels over time for IvoryEbony.
/// Prints envelope stage and level every 10ms to find discontinuities.
use xdx_core::sysex::dx100_decode_1voice;

// Minimal single-operator envelope (mirrors xdx-synth logic exactly)
#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Attack,
    Decay1,
    Decay2,
    Release,
    Off,
}

struct Env {
    stage: Stage,
    level: f32,
    ar_inc: f32,
    d1r_mul: f32,
    d2r_mul: f32,
    d1l: f32,
}

fn rate_inc(r: u8, max: u8, sr: f32) -> f32 {
    if r == 0 {
        return 0.0;
    }
    let t = 0.000085_f32 * 2.0_f32.powf((max as f32 - r as f32) * 0.55);
    1.0 / (t * sr)
}
fn rate_mul(r: u8, max: u8, sr: f32) -> f32 {
    if r == 0 {
        return 1.0;
    }
    let t = 0.0005_f32 * 2.0_f32.powf((max as f32 - r as f32) * 0.55);
    (-std::f32::consts::LN_2 / (t * sr)).exp()
}

impl Env {
    fn new(op: &xdx_core::dx100::Dx100Operator, sr: f32) -> Self {
        let d1l = if op.d1l == 0 {
            0.0
        } else if op.d1l >= 15 {
            1.0
        } else {
            2.0_f32.powf(op.d1l as f32 - 15.0)
        };
        Env {
            stage: Stage::Attack,
            level: 0.0,
            ar_inc: rate_inc(op.ar, 31, sr),
            d1r_mul: rate_mul(op.d1r, 31, sr),
            d2r_mul: rate_mul(op.d2r, 31, sr),
            d1l,
        }
    }
    fn tick(&mut self) {
        match self.stage {
            Stage::Attack => {
                self.level = (self.level + self.ar_inc).min(1.0);
                if self.level >= 1.0 {
                    self.stage = Stage::Decay1;
                }
            }
            Stage::Decay1 => {
                self.level = (self.level * self.d1r_mul).max(self.d1l);
                if self.level <= self.d1l + 1e-6 {
                    self.stage = Stage::Decay2;
                }
            }
            Stage::Decay2 => {
                self.level = (self.level * self.d2r_mul).max(0.0);
                if self.level < 1e-5 && self.d2r_mul < 1.0 {
                    self.stage = Stage::Off;
                }
            }
            _ => {}
        }
    }
}

fn level_to_amp(level: u8) -> f32 {
    if level == 0 {
        return 0.0;
    }
    let db = (level as f32 - 99.0) * 0.75;
    10f32.powf(db / 20.0)
}

fn main() {
    const SR: f32 = 44100.0;
    const MIDI_NOTE: u8 = 69;

    let bytes = std::fs::read("testdata/syx/IvoryEbony.syx").expect("read");
    let voice = dx100_decode_1voice(&bytes).expect("decode");

    println!("IvoryEbony EG trace (midi={MIDI_NOTE}, SR={SR})");
    println!(
        "{:>8}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}  {:>8}  note",
        "t_ms", "st1", "lv1", "st2", "lv2", "st3", "st4"
    );
    println!("{}", "-".repeat(72));

    let base_hz = 440.0_f32
        * 2.0_f32.powf((MIDI_NOTE as f32 + (voice.transpose as f32 - 24.0) - 69.0) / 12.0);

    // Build envelopes and compute attack-peak times analytically
    let mut envs: Vec<Env> = voice.ops.iter().map(|op| Env::new(op, SR)).collect();

    let print_interval = (10.0 * SR / 1000.0) as usize; // every 10ms
    let hold_samples = (2000.0 * SR / 1000.0) as usize; // 2s hold

    let mut prev_stages = [Stage::Attack; 4];

    for s in 0..hold_samples {
        for e in envs.iter_mut() {
            e.tick();
        }

        // Print on interval or stage transition
        let stage_changed = (0..4).any(|i| envs[i].stage != prev_stages[i]);
        if s % print_interval == 0 || stage_changed {
            let t_ms = s as f32 / SR * 1000.0;
            let marks: Vec<String> = (0..4)
                .map(|i| {
                    if envs[i].stage != prev_stages[i] {
                        format!(" OP{} {:?}→{:?}", i + 1, prev_stages[i], envs[i].stage)
                    } else {
                        String::new()
                    }
                })
                .collect();
            println!(
                "{:>8.1}  {:>6.4}  {:>6.4}  {:>6.4}  {:>6.4}{}",
                t_ms,
                envs[0].level,
                envs[1].level,
                envs[2].level,
                envs[3].level,
                marks.concat(),
            );
        }
        for i in 0..4 {
            prev_stages[i] = envs[i].stage;
        }
    }
    println!();

    // Print theoretical attack-peak time for each op
    println!("=== Theoretical attack-to-decay1 transitions ===");
    for (i, op) in voice.ops.iter().enumerate() {
        let ar_inc = rate_inc(op.ar, 31, SR);
        let peak_samples = if ar_inc > 0.0 {
            (1.0 / ar_inc).ceil() as u64
        } else {
            u64::MAX
        };
        let peak_ms = peak_samples as f64 / SR as f64 * 1000.0;
        let amp = level_to_amp(op.out_level);
        let ratio = FREQ_RATIOS[(op.freq_ratio as usize).min(63)];
        let det_cents = (op.detune as f32 - 3.0) * 3.0;
        let freq = base_hz * ratio * 2.0_f32.powf(det_cents / 1200.0);
        println!(
            "  OP{}: peak at {:.2}ms  amp={:.5}  freq={:.2}Hz  (ratio={} det={}ct)",
            i + 1,
            peak_ms,
            amp,
            freq,
            ratio,
            det_cents as i32
        );
    }
}
