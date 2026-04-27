use std::f32::consts::TAU;
use xdx_core::dx100::Dx100Voice;

use xdx_core::dx100::FREQ_RATIOS;

// ── Envelope ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Attack,
    Decay1,
    Decay2,
    Release,
    Off,
}

// DX100 EG attack uses a smoothstep S-curve: slow start → fast middle → slow near peak.
// Internally tracks normalised time ar_t ∈ [0, 1]; amplitude = smoothstep(ar_t).
#[derive(Clone)]
struct Envelope {
    stage: Stage,
    level: f32,    // 0.0..=1.0 linear amplitude (output)
    ar_inc_t: f32, // per-sample increment to normalised attack time
    ar_t: f32,     // normalised attack time 0..=1
    d1r_mul: f32,  // per-sample exponential multiplier (decay1)
    d2r_mul: f32,  // per-sample exponential multiplier (decay2)
    rr_mul: f32,   // per-sample exponential multiplier (release)
    d1l: f32,      // Decay-1 target level (linear, log-mapped)
}

impl Envelope {
    fn new() -> Self {
        Self {
            stage: Stage::Off,
            level: 0.0,
            ar_inc_t: 0.0,
            ar_t: 0.0,
            d1r_mul: 1.0,
            d2r_mul: 1.0,
            rr_mul: 1.0,
            d1l: 0.0,
        }
    }

    fn init(&mut self, op: &xdx_core::dx100::Dx100Operator, sr: f32, midi_note: u8) {
        // kbd_rate_scl 0-3: EG rate boost scaling over all notes.
        // Hardware calibration (kbs_calib, n48-n84):
        //   krs maps non-linearly: effective_krs ≈ krs*(krs+1)/2  = [0,1,3,6]
        //   boost ≈ effective_krs * note / 72  (normalised to C5 = MIDI 72)
        // No hard breakpoint — applies at all notes proportional to pitch.
        let effective_krs = (op.kbd_rate_scl * (op.kbd_rate_scl + 1)) / 2; // 0,1,3,6
        let rate_boost = (effective_krs as f32 * midi_note as f32 / 72.0).round() as u8;

        self.ar_inc_t = rate_inc_t((op.ar + rate_boost).min(31), 31, sr);
        self.ar_t = 0.0;
        self.d1r_mul = rate_mul((op.d1r + rate_boost).min(31), 31, 0.000092, sr);
        self.d2r_mul = rate_mul((op.d2r + rate_boost).min(31), 31, 0.000092, sr);
        self.rr_mul = rate_mul((op.rr + rate_boost).min(15), 15, 0.0014, sr);
        // DX100: D1L 0-15 uses 3 dB per step (√2 factor per step)
        self.d1l = if op.d1l == 0 {
            0.0
        } else if op.d1l >= 15 {
            1.0
        } else {
            2.0_f32.powf((op.d1l as f32 - 15.0) * 0.5)
        };
        self.level = 0.0;
        self.stage = Stage::Attack;
    }

    fn release(&mut self) {
        if self.stage != Stage::Off {
            self.stage = Stage::Release;
        }
    }

    #[inline]
    fn tick(&mut self) -> f32 {
        match self.stage {
            Stage::Attack => {
                if self.ar_inc_t > 0.0 {
                    self.ar_t = (self.ar_t + self.ar_inc_t).min(1.0);
                    // smoothstep: level = 3t² - 2t³  (slow start, fast middle, slow end)
                    self.level = self.ar_t * self.ar_t * (3.0 - 2.0 * self.ar_t);
                    if self.ar_t >= 1.0 {
                        self.level = 1.0;
                        self.stage = Stage::Decay1;
                    }
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
            Stage::Release => {
                self.level = (self.level * self.rr_mul).max(0.0);
                if self.level < 1e-5 {
                    self.stage = Stage::Off;
                }
            }
            Stage::Off => {}
        }
        self.level
    }

    fn is_off(&self) -> bool {
        self.stage == Stage::Off
    }
}

// Attack: per-sample increment to normalised attack time (smoothstep S-curve).
// t = full attack duration; calibrated from DX100 hardware: AR=20 → atk90 ≈ 5ms.
fn rate_inc_t(rate: u8, max_rate: u8, sr: f32) -> f32 {
    if rate == 0 {
        return 0.0;
    }
    let t = 0.000085_f32 * 2.0_f32.powf((max_rate as f32 - rate as f32) * 0.55);
    1.0 / (t * sr)
}

// Decay/Release: exponential (multiplicative) per-sample factor.
// coeff calibrated from hardware: D1R/D2R=0.000092, RR=0.0014.  rate=0 → no decay.
fn rate_mul(rate: u8, max_rate: u8, coeff: f32, sr: f32) -> f32 {
    if rate == 0 {
        return 1.0;
    }
    let t = coeff * 2.0_f32.powf((max_rate as f32 - rate as f32) * 0.55);
    (-std::f32::consts::LN_2 / (t * sr)).exp()
}

// ── LFO ──────────────────────────────────────────────────────────────────────

// DX100 hardware-measured LFO speed: piecewise linear between calibration points.
// speed=0 extrapolated from "period ~16s" note in recordings (undetectable in 8s window).
// speed=5 estimated: HW shows ~4× fewer oscillations than linear interpolation predicts
//   (PMD50_S7 comparison, hold=8s); ~0.13 Hz. Needs precise re-measurement.
const LFO_SPEED_TABLE: [(f32, f32); 8] = [
    (0.0, 0.063),
    (5.0, 0.13), // estimated; linear interpolation overshoots ~4×
    (16.0, 1.511),
    (33.0, 6.183),
    (50.0, 13.214),
    (66.0, 24.849),
    (83.0, 39.253),
    (99.0, 52.946),
];

fn lfo_speed_hz(speed: u8) -> f32 {
    let s = speed as f32;
    let n = LFO_SPEED_TABLE.len();
    if s <= LFO_SPEED_TABLE[0].0 {
        return LFO_SPEED_TABLE[0].1;
    }
    if s >= LFO_SPEED_TABLE[n - 1].0 {
        return LFO_SPEED_TABLE[n - 1].1;
    }
    for i in 1..n {
        if s <= LFO_SPEED_TABLE[i].0 {
            let (s0, h0) = LFO_SPEED_TABLE[i - 1];
            let (s1, h1) = LFO_SPEED_TABLE[i];
            let t = (s - s0) / (s1 - s0);
            return h0 + t * (h1 - h0);
        }
    }
    LFO_SPEED_TABLE[n - 1].1
}

// DX100 lfo_delay → samples of silence before LFO becomes audible.
// Encoding: a = (16 + (delay & 15)) << (1 + (delay >> 4))
// Measured onset: onset_ms ≈ 1.406 × a^1.166 (3 DX100 data points; ±30% accuracy).
fn lfo_delay_samples(delay: u8, sr: f32) -> u32 {
    if delay == 0 {
        return 0;
    }
    let a = ((16 + (delay & 15) as u32) << (1 + (delay >> 4) as u32)) as f32;
    let ms = 1.406_f32 * a.powf(1.166);
    (ms / 1000.0 * sr) as u32
}

// PMS 0-7 → max pitch deviation in cents at PMD=99, LFO=+1.0.
// Anchored at PMS=3→20¢ and PMS=7→700¢ from DX100 hardware measurements.
// Intermediate values interpolated geometrically (~2.43× per step).
const PMS_CENTS: [f32; 8] = [0.0, 3.4, 8.2, 20.0, 48.6, 118.0, 287.0, 700.0];

// AMS 0-3 → max AM attenuation depth in dB at AMD=99, LFO peak (+1.0).
// AMS=1..3 all hit the ≈48 dB noise floor at AMD=99 (saturated); ratios
// between steps are unverified at lower AMD and assumed to double each step.
const AMS_DB: [f32; 4] = [0.0, 48.0, 96.0, 192.0];

#[derive(Clone)]
struct Lfo {
    phase: f32,
    hz: f32,
    amplitude: f32, // 0.0..1.0, ramps up from zero after delay ends
    elapsed: u32,   // samples elapsed since note-on
    delay_samples: u32,
    ramp_samples: u32, // linear ramp-up duration after delay; 500ms when delay > 0
    s_h_value: f32,    // S&H held value
}

impl Lfo {
    fn new(voice: &Dx100Voice, sr: f32) -> Self {
        let hz = lfo_speed_hz(voice.lfo_speed);
        let delay_samples = lfo_delay_samples(voice.lfo_delay, sr);
        // 500ms linear ramp after delay; no ramp when delay=0 (start at full amplitude).
        let ramp_samples = if delay_samples > 0 {
            (0.5 * sr) as u32
        } else {
            0
        };
        // SYNC=1 resets LFO phase at note-on.
        // DX100 TRI starts at lfo_out=0 (center, ascending) → phase=0.25.
        // SAW/SQU start at lfo_out=+1 (positive peak) → phase=0.0.
        let phase = if voice.lfo_sync != 0 && voice.lfo_wave == 2 {
            0.25
        } else {
            0.0
        };
        Self {
            phase,
            hz,
            amplitude: if delay_samples == 0 { 1.0 } else { 0.0 },
            elapsed: 0,
            delay_samples,
            ramp_samples,
            s_h_value: 0.0,
        }
    }

    #[inline]
    fn tick(&mut self, sr: f32, wave: u8) -> f32 {
        self.elapsed = self.elapsed.saturating_add(1);
        if self.elapsed <= self.delay_samples {
            self.amplitude = 0.0;
        } else {
            let after = self.elapsed - self.delay_samples;
            self.amplitude = if self.ramp_samples > 0 {
                (after as f32 / self.ramp_samples as f32).min(1.0)
            } else {
                1.0
            };
        }

        self.phase += self.hz / sr;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            if wave == 3 {
                // LCG hash of elapsed for S&H pseudo-random output
                let h = self
                    .elapsed
                    .wrapping_mul(2891336453)
                    .wrapping_add(1640531527);
                self.s_h_value = (h as f32 / u32::MAX as f32) * 2.0 - 1.0;
            }
        }

        let raw = match wave {
            0 => 1.0 - 2.0 * self.phase, // SAW descending
            1 => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            } // SQU
            2 => {
                // TRI
                if self.phase < 0.5 {
                    4.0 * self.phase - 1.0
                } else {
                    3.0 - 4.0 * self.phase
                }
            }
            _ => self.s_h_value, // S&H
        };
        raw * self.amplitude
    }
}

// ── Operator ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Operator {
    phase: f64,
    env: Envelope,
    // Computed at note-on
    freq: f32,        // absolute frequency (Hz)
    amp: f32,         // 0..1 linear from out_level
    amp_mod_en: bool, // true → operator output is attenuated by LFO AM
}

impl Operator {
    fn new() -> Self {
        Self {
            phase: 0.0,
            env: Envelope::new(),
            freq: 0.0,
            amp: 0.0,
            amp_mod_en: false,
        }
    }

    /// Advance phase and return output sample (in radians for FM modulation).
    /// `mod_input` is total phase modulation from upstream operators (radians).
    /// `pitch_ratio` scales frequency from LFO pitch modulation.
    #[inline]
    fn tick(&mut self, sr: f32, mod_input: f32, pitch_ratio: f32) -> f32 {
        self.phase += (self.freq * pitch_ratio) as f64 / sr as f64;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let env = self.env.tick();
        (self.phase as f32 * TAU + mod_input).sin() * env * self.amp
    }
}

// ── Note ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Note {
    midi_note: u8,
    ops: [Operator; 4], // ops[0]=OP1(feedback), ops[1]=OP2, ops[2]=OP3, ops[3]=OP4
    fb_prev: f32,       // OP4 output at t-1 (DX100: feedback is always on OP4)
    fb_prev2: f32,      // OP4 output at t-2
    lfo: Lfo,
}

impl Note {
    fn start(midi_note: u8, velocity: u8, voice: &Dx100Voice, sr: f32) -> Self {
        let base_hz = midi_to_hz(midi_note, voice.transpose);
        let vel_scale = (velocity as f32 / 127.0).powi(2);
        // kbd_lev_scl: exponential scaling over all notes.
        // Hardware calibration shows reduction doubles per octave:
        //   reduction = floor(kls * 2^(note/12) / 400)
        // K=1/400 derived from DX100 recordings (kls=25/50, notes 48/60/72/84).
        let kls_note_factor = 2.0_f32.powf(midi_note as f32 / 12.0);
        let mut ops = [
            Operator::new(),
            Operator::new(),
            Operator::new(),
            Operator::new(),
        ];
        for (i, op) in ops.iter_mut().enumerate() {
            let p = &voice.ops[i];
            let ratio = FREQ_RATIOS[(p.freq_ratio as usize).min(63)];
            let detune_cents = (p.detune as f32 - 3.0) * 3.0;
            op.freq = base_hz * ratio * 2.0_f32.powf(detune_cents / 1200.0);
            let kls_reduction = (p.kbd_lev_scl as f32 * kls_note_factor / 400.0) as u8;
            let vel_factor = 1.0 - (p.key_vel_sens as f32 / 7.0) * (1.0 - vel_scale);
            op.amp = level_to_amp(p.out_level.saturating_sub(kls_reduction)) * vel_factor;
            op.env.init(p, sr, midi_note);
            op.amp_mod_en = p.amp_mod_en != 0;
        }
        Note {
            midi_note,
            ops,
            fb_prev: 0.0,
            fb_prev2: 0.0,
            lfo: Lfo::new(voice, sr),
        }
    }

    fn release(&mut self) {
        for op in &mut self.ops {
            op.env.release();
        }
    }

    fn is_off(&self) -> bool {
        self.ops.iter().all(|o| o.env.is_off())
    }

    #[inline]
    fn render_sample(&mut self, algo: u8, fb_depth: f32, sr: f32, voice: &Dx100Voice) -> f32 {
        let lfo_out = self.lfo.tick(sr, voice.lfo_wave);

        // Pitch modulation: uniform frequency shift across all operators.
        let pitch_cents =
            PMS_CENTS[voice.pitch_mod_sens as usize & 7] * (voice.lfo_pmd as f32 / 99.0) * lfo_out;
        let pitch_ratio = 2.0_f32.powf(pitch_cents / 1200.0);

        // Amplitude modulation: LFO remapped to 0..1 (negative half = no attenuation).
        // Log-domain AM matches DX100 hardware: all AMS values saturate at ≈48 dB at AMD=99.
        let lfo_am = lfo_out * 0.5 + 0.5;
        let attenuation_db =
            lfo_am * (voice.lfo_amd as f32 / 99.0) * AMS_DB[(voice.amp_mod_sens as usize) & 3];
        let am_on = 10.0_f32.powf(-attenuation_db / 20.0);

        // Pre-read amp_mod_en flags before mutable operator borrows.
        let am_en = [
            self.ops[0].amp_mod_en,
            self.ops[1].amp_mod_en,
            self.ops[2].amp_mod_en,
            self.ops[3].amp_mod_en,
        ];
        let am = |en: bool| -> f32 {
            if en {
                am_on
            } else {
                1.0
            }
        };

        // Evaluate operators in modulator-first order for each algorithm.
        // ops[0]=OP1, [1]=OP2, [2]=OP3, [3]=OP4
        //
        // DX100: feedback is always on OP4 (self-modulation).
        // Averages the last two OP4 outputs to stabilise high-feedback chaos.
        // Modulator outputs are scaled by TAU (≈±2π rad at full level) before
        // being added to the carrier phase, matching Yamaha hardware.
        // fb_prev tracks the pre-AM OP4 value to keep the feedback loop stable.
        let fb_mod = (self.fb_prev + self.fb_prev2) * 0.5 * fb_depth;

        match algo {
            // ── 1 carrier ──────────────────────────────────────────────────
            // Alg 0: OP4(fb)→OP3→OP2→OP1(C)
            0 => {
                let o4r = self.ops[3].tick(sr, fb_mod, pitch_ratio);
                let o4 = o4r * am(am_en[3]);
                let o3 = self.ops[2].tick(sr, o4 * TAU, pitch_ratio) * am(am_en[2]);
                let o2 = self.ops[1].tick(sr, o3 * TAU, pitch_ratio) * am(am_en[1]);
                let o1 = self.ops[0].tick(sr, o2 * TAU, pitch_ratio) * am(am_en[0]);
                self.fb_prev2 = self.fb_prev;
                self.fb_prev = o4r;
                o1
            }
            // Alg 1: [OP4(fb)→OP3, OP2] → OP1(C)
            1 => {
                let o4r = self.ops[3].tick(sr, fb_mod, pitch_ratio);
                let o4 = o4r * am(am_en[3]);
                let o2 = self.ops[1].tick(sr, 0.0, pitch_ratio) * am(am_en[1]);
                let o3 = self.ops[2].tick(sr, o4 * TAU, pitch_ratio) * am(am_en[2]);
                let o1 = self.ops[0].tick(sr, (o3 + o2) * TAU, pitch_ratio) * am(am_en[0]);
                self.fb_prev2 = self.fb_prev;
                self.fb_prev = o4r;
                o1
            }
            // Alg 2: OP4(fb)→[OP3+OP2]→OP1(C)
            2 => {
                let o4r = self.ops[3].tick(sr, fb_mod, pitch_ratio);
                let o4 = o4r * am(am_en[3]);
                let o3 = self.ops[2].tick(sr, o4 * TAU, pitch_ratio) * am(am_en[2]);
                let o2 = self.ops[1].tick(sr, o4 * TAU, pitch_ratio) * am(am_en[1]);
                let o1 = self.ops[0].tick(sr, (o3 + o2) * TAU, pitch_ratio) * am(am_en[0]);
                self.fb_prev2 = self.fb_prev;
                self.fb_prev = o4r;
                o1
            }
            // Alg 3: [OP4(fb)+OP3+OP2]→OP1(C)
            3 => {
                let o4r = self.ops[3].tick(sr, fb_mod, pitch_ratio);
                let o4 = o4r * am(am_en[3]);
                let o3 = self.ops[2].tick(sr, 0.0, pitch_ratio) * am(am_en[2]);
                let o2 = self.ops[1].tick(sr, 0.0, pitch_ratio) * am(am_en[1]);
                let o1 = self.ops[0].tick(sr, (o4 + o3 + o2) * TAU, pitch_ratio) * am(am_en[0]);
                self.fb_prev2 = self.fb_prev;
                self.fb_prev = o4r;
                o1
            }
            // ── 2 carriers ─────────────────────────────────────────────────
            // Alg 4: [OP4(fb)→OP3(C)] + [OP2→OP1(C)]
            4 => {
                let o4r = self.ops[3].tick(sr, fb_mod, pitch_ratio);
                let o4 = o4r * am(am_en[3]);
                let o2 = self.ops[1].tick(sr, 0.0, pitch_ratio) * am(am_en[1]);
                let o3 = self.ops[2].tick(sr, o4 * TAU, pitch_ratio) * am(am_en[2]);
                let o1 = self.ops[0].tick(sr, o2 * TAU, pitch_ratio) * am(am_en[0]);
                self.fb_prev2 = self.fb_prev;
                self.fb_prev = o4r;
                (o3 + o1) * 0.5
            }
            // ── 3 carriers ─────────────────────────────────────────────────
            // Alg 5: OP4(fb)→[OP3(C)+OP2(C)+OP1(C)]
            5 => {
                let o4r = self.ops[3].tick(sr, fb_mod, pitch_ratio);
                let o4 = o4r * am(am_en[3]);
                let o3 = self.ops[2].tick(sr, o4 * TAU, pitch_ratio) * am(am_en[2]);
                let o2 = self.ops[1].tick(sr, o4 * TAU, pitch_ratio) * am(am_en[1]);
                let o1 = self.ops[0].tick(sr, o4 * TAU, pitch_ratio) * am(am_en[0]);
                self.fb_prev2 = self.fb_prev;
                self.fb_prev = o4r;
                (o3 + o2 + o1) / 3.0
            }
            // Alg 6: [OP4(fb)→OP3(C)] + OP2(C) + OP1(C)
            6 => {
                let o4r = self.ops[3].tick(sr, fb_mod, pitch_ratio);
                let o4 = o4r * am(am_en[3]);
                let o3 = self.ops[2].tick(sr, o4 * TAU, pitch_ratio) * am(am_en[2]);
                let o2 = self.ops[1].tick(sr, 0.0, pitch_ratio) * am(am_en[1]);
                let o1 = self.ops[0].tick(sr, 0.0, pitch_ratio) * am(am_en[0]);
                self.fb_prev2 = self.fb_prev;
                self.fb_prev = o4r;
                (o3 + o2 + o1) / 3.0
            }
            // ── 4 carriers (pure additive) ──────────────────────────────────
            // Alg 7: OP4(fb,C)+OP3(C)+OP2(C)+OP1(C)
            _ => {
                let o4r = self.ops[3].tick(sr, fb_mod, pitch_ratio);
                let o4 = o4r * am(am_en[3]);
                let o3 = self.ops[2].tick(sr, 0.0, pitch_ratio) * am(am_en[2]);
                let o2 = self.ops[1].tick(sr, 0.0, pitch_ratio) * am(am_en[1]);
                let o1 = self.ops[0].tick(sr, 0.0, pitch_ratio) * am(am_en[0]);
                self.fb_prev2 = self.fb_prev;
                self.fb_prev = o4r;
                (o4 + o3 + o2 + o1) * 0.25
            }
        }
    }
}

// ── Engine ────────────────────────────────────────────────────────────────────

pub struct FmEngine {
    voice: Dx100Voice,
    sample_rate: f32,
    notes: Vec<Note>,
    fb_depth: f32, // computed from voice.feedback
}

impl FmEngine {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voice: Dx100Voice::default(),
            sample_rate,
            notes: Vec::new(),
            fb_depth: 0.0,
        }
    }

    pub fn set_voice(&mut self, voice: Dx100Voice) {
        self.fb_depth = feedback_depth(voice.feedback);
        self.voice = voice;
    }

    pub fn note_on(&mut self, midi_note: u8, velocity: u8) {
        // Drop any existing note with the same pitch
        self.notes.retain(|n| n.midi_note != midi_note);
        self.notes.push(Note::start(
            midi_note,
            velocity,
            &self.voice,
            self.sample_rate,
        ));
    }

    pub fn note_off(&mut self, midi_note: u8) {
        for n in &mut self.notes {
            if n.midi_note == midi_note {
                n.release();
                break;
            }
        }
    }

    pub fn render(&mut self, buf: &mut [f32]) {
        let algo = self.voice.algorithm;
        let fb_depth = self.fb_depth;
        let sr = self.sample_rate;
        let voice = &self.voice;

        for sample in buf.iter_mut() {
            let mut out = 0.0_f32;
            for note in &mut self.notes {
                out += note.render_sample(algo, fb_depth, sr, voice);
            }
            *sample = out;
        }
        // Retire finished notes
        self.notes.retain(|n| !n.is_off());
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn midi_to_hz(note: u8, transpose: u8) -> f32 {
    // transpose: 0-48, center=24 → offset in semitones
    let semitone = note as f32 + (transpose as f32 - 24.0);
    440.0 * 2.0_f32.powf((semitone - 69.0) / 12.0)
}

fn level_to_amp(level: u8) -> f32 {
    if level == 0 {
        return 0.0;
    }
    // Yamaha DX spec: 0.75 dB per step, 0 dB at level 99
    let db = (level as f32 - 99.0) * 0.75;
    10f32.powf(db / 20.0)
}

// feedback 0-7 → phase modulation depth (radians).
// DX100/DX7: fb=7 → π rad (creates sawtooth-like carrier); each step halves the depth.
fn feedback_depth(fb: u8) -> f32 {
    if fb == 0 {
        return 0.0;
    }
    std::f32::consts::PI * 2.0_f32.powf(fb as f32 - 7.0)
}
