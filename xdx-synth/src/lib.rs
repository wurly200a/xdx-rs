use std::f32::consts::TAU;
use xdx_core::dx100::Dx100Voice;

// ── Frequency ratio table (freq_ratio 0-63) ───────────────────────────────────
// From DX100 documentation. Low indices include sub-harmonics and sqrt(2) steps.
// From dx100ParamCtrl.c (authoritative source)
const FREQ_RATIOS: [f32; 64] = [
     0.50,  0.71,  0.78,  0.87,  1.00,  1.41,  1.57,  1.73,
     2.00,  2.82,  3.00,  3.14,  3.46,  4.00,  4.24,  4.71,
     5.00,  5.19,  5.65,  6.00,  6.28,  6.92,  7.00,  7.07,
     7.85,  8.00,  8.48,  8.65,  9.00,  9.42,  9.89, 10.00,
    10.38, 10.99, 11.00, 11.30, 12.00, 12.11, 12.56, 12.72,
    13.00, 13.84, 14.00, 14.10, 14.13, 15.00, 15.55, 15.57,
    15.70, 16.96, 17.27, 17.30, 18.37, 18.84, 19.03, 19.78,
    20.41, 20.76, 21.20, 21.98, 22.49, 23.55, 24.22, 25.95,
];

// ── Envelope ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Stage { Attack, Decay1, Decay2, Release, Off }

#[derive(Clone)]
struct Envelope {
    stage:   Stage,
    level:   f32,   // 0.0..=1.0
    // Per-sample increments / targets (computed on note-on)
    ar_inc:  f32,
    d1r_inc: f32,
    d2r_inc: f32,
    rr_inc:  f32,
    d1l:     f32,   // Decay-1 target level (0.0..=1.0)
}

impl Envelope {
    fn new() -> Self {
        Self { stage: Stage::Off, level: 0.0, ar_inc: 0.0, d1r_inc: 0.0,
               d2r_inc: 0.0, rr_inc: 0.0, d1l: 0.0 }
    }

    fn init(&mut self, op: &xdx_core::dx100::Dx100Operator, sr: f32) {
        self.ar_inc  = rate_inc(op.ar,  31, sr);
        self.d1r_inc = rate_inc(op.d1r, 31, sr);
        self.d2r_inc = rate_inc(op.d2r, 31, sr);
        self.rr_inc  = rate_inc(op.rr,  15, sr);
        self.d1l     = op.d1l as f32 / 15.0;
        self.level   = 0.0;
        self.stage   = Stage::Attack;
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
                self.level = (self.level + self.ar_inc).min(1.0);
                if self.level >= 1.0 {
                    self.stage = Stage::Decay1;
                }
            }
            Stage::Decay1 => {
                self.level = (self.level - self.d1r_inc).max(self.d1l);
                if self.level <= self.d1l {
                    self.stage = Stage::Decay2;
                }
            }
            Stage::Decay2 => {
                self.level = (self.level - self.d2r_inc).max(0.0);
            }
            Stage::Release => {
                self.level = (self.level - self.rr_inc).max(0.0);
                if self.level <= 0.0 {
                    self.stage = Stage::Off;
                }
            }
            Stage::Off => {}
        }
        self.level
    }

    fn is_off(&self) -> bool { self.stage == Stage::Off }
}

// Convert an envelope rate (0..max_rate) to a per-sample linear increment.
// rate=max_rate → ~0.5 ms; rate=0 → very slow (rate=0 treated as hold/no decay).
fn rate_inc(rate: u8, max_rate: u8, sr: f32) -> f32 {
    if rate == 0 { return 0.0; }
    // time_s = 0.0005 * 2^((max_rate - rate) * 0.55)
    let t = 0.0005_f32 * 2.0_f32.powf((max_rate as f32 - rate as f32) * 0.55);
    1.0 / (t * sr)
}

// ── Operator ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Operator {
    phase:  f64,
    env:    Envelope,
    // Computed at note-on
    freq:   f32,   // absolute frequency (Hz)
    amp:    f32,   // 0..1 linear from out_level
}

impl Operator {
    fn new() -> Self {
        Self { phase: 0.0, env: Envelope::new(), freq: 0.0, amp: 0.0 }
    }

    /// Advance phase and return output sample (in radians for FM modulation).
    /// `mod_input` is total phase modulation from upstream operators (radians).
    #[inline]
    fn tick(&mut self, sr: f32, mod_input: f32) -> f32 {
        self.phase += self.freq as f64 / sr as f64;
        if self.phase >= 1.0 { self.phase -= 1.0; }
        let env = self.env.tick();
        (self.phase as f32 * TAU + mod_input).sin() * env * self.amp
    }
}

// ── Note ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Note {
    midi_note: u8,
    ops:       [Operator; 4],  // ops[0]=OP1(feedback), ops[1]=OP2, ops[2]=OP3, ops[3]=OP4
    fb_prev:   f32,            // OP1 previous output for feedback
}

impl Note {
    fn start(midi_note: u8, velocity: u8, voice: &Dx100Voice, sr: f32) -> Self {
        let base_hz = midi_to_hz(midi_note, voice.transpose);
        let vel_scale = (velocity as f32 / 127.0).powi(2);
        let mut ops = [
            Operator::new(), Operator::new(), Operator::new(), Operator::new(),
        ];
        for (i, op) in ops.iter_mut().enumerate() {
            let p = &voice.ops[i];
            let ratio = FREQ_RATIOS[(p.freq_ratio as usize).min(63)];
            let detune_cents = (p.detune as f32 - 3.0) * 3.0;
            op.freq = base_hz * ratio * 2.0_f32.powf(detune_cents / 1200.0);
            // Velocity sensitivity: blend out_level with velocity
            let vel_factor = 1.0 - (p.key_vel_sens as f32 / 7.0) * (1.0 - vel_scale);
            op.amp  = level_to_amp(p.out_level) * vel_factor;
            op.env.init(p, sr);
        }
        Note { midi_note, ops, fb_prev: 0.0 }
    }

    fn release(&mut self) {
        for op in &mut self.ops { op.env.release(); }
    }

    fn is_off(&self) -> bool {
        self.ops.iter().all(|o| o.env.is_off())
    }

    #[inline]
    fn render_sample(&mut self, algo: u8, fb_depth: f32, sr: f32) -> f32 {
        // Evaluate operators in modulator-first order for each algorithm.
        // ops[0]=OP1(feedback), [1]=OP2, [2]=OP3, [3]=OP4
        //
        // Modulation depth scaling: operator output is in [-amp, +amp] where
        // amp ≤ 1.0.  Yamaha FM hardware scales this to ≈ ±2π radians at full
        // level before adding to the carrier phase.  We replicate that here by
        // multiplying every modulator→carrier connection by TAU.
        let fb_mod = self.fb_prev * fb_depth;

        match algo {
            // ── 1 carrier ──────────────────────────────────────────────────
            // Alg 0: OP4→OP3→OP2→OP1(C)
            0 => {
                let o4 = self.ops[3].tick(sr, 0.0);
                let o3 = self.ops[2].tick(sr, o4 * TAU);
                let o2 = self.ops[1].tick(sr, o3 * TAU);
                let o1 = self.ops[0].tick(sr, o2 * TAU + fb_mod);
                self.fb_prev = o1;
                o1
            }
            // Alg 1: [OP4→OP3, OP2] → OP1(C)
            1 => {
                let o4 = self.ops[3].tick(sr, 0.0);
                let o2 = self.ops[1].tick(sr, 0.0);
                let o3 = self.ops[2].tick(sr, o4 * TAU);
                let o1 = self.ops[0].tick(sr, (o3 + o2) * TAU + fb_mod);
                self.fb_prev = o1;
                o1
            }
            // Alg 2: OP4→[OP3+OP2]→OP1(C)
            2 => {
                let o4 = self.ops[3].tick(sr, 0.0);
                let o3 = self.ops[2].tick(sr, o4 * TAU);
                let o2 = self.ops[1].tick(sr, o4 * TAU);
                let o1 = self.ops[0].tick(sr, (o3 + o2) * TAU + fb_mod);
                self.fb_prev = o1;
                o1
            }
            // Alg 3: [OP4+OP3+OP2]→OP1(C)
            3 => {
                let o4 = self.ops[3].tick(sr, 0.0);
                let o3 = self.ops[2].tick(sr, 0.0);
                let o2 = self.ops[1].tick(sr, 0.0);
                let o1 = self.ops[0].tick(sr, (o4 + o3 + o2) * TAU + fb_mod);
                self.fb_prev = o1;
                o1
            }
            // ── 2 carriers ─────────────────────────────────────────────────
            // Alg 4: [OP4→OP3(C)] + [OP2→OP1(C)]
            4 => {
                let o4 = self.ops[3].tick(sr, 0.0);
                let o2 = self.ops[1].tick(sr, 0.0);
                let o3 = self.ops[2].tick(sr, o4 * TAU);
                let o1 = self.ops[0].tick(sr, o2 * TAU + fb_mod);
                self.fb_prev = o1;
                (o3 + o1) * 0.5
            }
            // ── 3 carriers ─────────────────────────────────────────────────
            // Alg 5: OP4→[OP3(C)+OP2(C)+OP1(C)]
            5 => {
                let o4 = self.ops[3].tick(sr, 0.0);
                let o3 = self.ops[2].tick(sr, o4 * TAU);
                let o2 = self.ops[1].tick(sr, o4 * TAU);
                let o1 = self.ops[0].tick(sr, o4 * TAU + fb_mod);
                self.fb_prev = o1;
                (o3 + o2 + o1) / 3.0
            }
            // Alg 6: [OP4→OP3(C)] + OP2(C) + OP1(C)
            6 => {
                let o4 = self.ops[3].tick(sr, 0.0);
                let o3 = self.ops[2].tick(sr, o4 * TAU);
                let o2 = self.ops[1].tick(sr, 0.0);
                let o1 = self.ops[0].tick(sr, fb_mod);
                self.fb_prev = o1;
                (o3 + o2 + o1) / 3.0
            }
            // ── 4 carriers (pure additive) ──────────────────────────────────
            // Alg 7: OP4(C)+OP3(C)+OP2(C)+OP1(C)
            _ => {
                let o4 = self.ops[3].tick(sr, 0.0);
                let o3 = self.ops[2].tick(sr, 0.0);
                let o2 = self.ops[1].tick(sr, 0.0);
                let o1 = self.ops[0].tick(sr, fb_mod);
                self.fb_prev = o1;
                (o4 + o3 + o2 + o1) * 0.25
            }
        }
    }
}

// ── Engine ────────────────────────────────────────────────────────────────────

pub struct FmEngine {
    voice:       Dx100Voice,
    sample_rate: f32,
    notes:       Vec<Note>,
    fb_depth:    f32,   // computed from voice.feedback
}

impl FmEngine {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voice:       Dx100Voice::default(),
            sample_rate,
            notes:       Vec::new(),
            fb_depth:    0.0,
        }
    }

    pub fn set_voice(&mut self, voice: Dx100Voice) {
        self.fb_depth = feedback_depth(voice.feedback);
        self.voice = voice;
    }

    pub fn note_on(&mut self, midi_note: u8, velocity: u8) {
        // Drop any existing note with the same pitch
        self.notes.retain(|n| n.midi_note != midi_note);
        self.notes.push(Note::start(midi_note, velocity, &self.voice, self.sample_rate));
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
        let algo     = self.voice.algorithm;
        let fb_depth = self.fb_depth;
        let sr       = self.sample_rate;

        for sample in buf.iter_mut() {
            let mut out = 0.0_f32;
            for note in &mut self.notes {
                out += note.render_sample(algo, fb_depth, sr);
            }
            // Normalize by number of concurrent notes to prevent clipping
            *sample = if self.notes.is_empty() { 0.0 } else { out / self.notes.len() as f32 };
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
    // out_level 0-99 → amplitude 0..1 with logarithmic-feeling curve
    (level as f32 / 99.0).powi(2)
}

// feedback 0-7 → phase modulation depth (radians).
// DX7/DX100: 0=no feedback, 7=strong self-modulation.
fn feedback_depth(fb: u8) -> f32 {
    if fb == 0 { return 0.0; }
    // Each step roughly doubles the depth; ~π/4 at max
    0.04 * 2.0_f32.powf(fb as f32)
}
