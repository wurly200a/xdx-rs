//! xdx-compare — DX100 vs softsynth waveform capture tool
//!
//! Usage:
//!   xdx-compare --list
//!   xdx-compare <voice.syx> [midi_note=69] [options]
//!
//! Options:
//!   --list                  List available MIDI OUT and audio IN devices
//!   --midi-out <name>       MIDI OUT port connected to DX100
//!   --audio-in <name>       Audio input device connected to DX100 output
//!   --duration <secs>       Note hold duration  (default: 2.0)
//!   --release <secs>        Release capture time (default: 1.0)
//!   --channel <1-16>        MIDI channel        (default: 1)
//!   --out <dir>             Output directory    (default: .)
//!
//! Outputs:
//!   <dir>/synth.wav   — softsynth render
//!   <dir>/dx100.wav   — DX100 recording (only when --midi-out + --audio-in given)
//!   <dir>/metrics.txt — basic level/duration stats

use std::sync::{Arc, Mutex};
use std::time::Duration;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use xdx_midi::MidiManager;
use xdx_synth::FmEngine;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--list") {
        list_devices();
        return;
    }

    let syx_path = args.get(1)
        .expect("Usage: xdx-compare <voice.syx> [midi_note] [options]\n       xdx-compare --list");
    let midi_note: u8 = args.get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(69);

    let midi_out  = flag_val(&args, "--midi-out");
    let audio_in  = flag_val(&args, "--audio-in");
    let duration: f32 = flag_val(&args, "--duration")
        .and_then(|s| s.parse().ok()).unwrap_or(2.0);
    let release: f32  = flag_val(&args, "--release")
        .and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let channel: u8   = flag_val(&args, "--channel")
        .and_then(|s| s.parse::<u8>().ok())
        .map(|c| c.clamp(1, 16))
        .unwrap_or(1);
    let out_dir = flag_val(&args, "--out").unwrap_or_else(|| ".".to_string());

    // Load voice
    let bytes = std::fs::read(syx_path)
        .unwrap_or_else(|e| panic!("Cannot read {syx_path}: {e}"));
    let voice = xdx_core::sysex::dx100_decode_1voice(&bytes)
        .unwrap_or_else(|e| panic!("Decode failed: {e:?}"));

    println!("=== xdx-compare ===");
    println!("Voice:     \"{}\"", voice.name_str());
    println!("MIDI note: {midi_note}  channel: {channel}");
    println!("Duration:  {duration:.1}s hold + {release:.1}s release");
    println!("Output:    {out_dir}/");

    std::fs::create_dir_all(&out_dir)
        .unwrap_or_else(|e| panic!("Cannot create output dir: {e}"));

    // ── 1. Render softsynth (to memory) ──────────────────────────────────────
    let (synth, synth_samples) = render_synth(&voice, midi_note, duration, release);
    println!("\n[synth raw]  sustain_rms={:.4}  peak={:.4}  crest={:.4}",
        synth.sustain_rms, synth.peak, synth.crest_factor);

    // ── 2. Record DX100 ───────────────────────────────────────────────────────
    if midi_out.is_some() && audio_in.is_some() {
        let (dx100, dx100_samples, dx100_sr) = record_dx100(
            midi_note, channel, duration, release,
            midi_out.as_deref().unwrap(),
            audio_in.as_deref().unwrap(),
        );

        // ── 3. Compute level match gain and save WAVs ─────────────────────────
        let gain = if synth.sustain_rms > 0.0 { dx100.sustain_rms / synth.sustain_rms } else { 1.0 };
        let gain_db = if gain > 0.0 { 20.0 * gain.log10() } else { f32::NEG_INFINITY };

        let synth_path = format!("{out_dir}/synth.wav");
        save_wav(&synth_path, &synth_samples, 44100, gain);
        let dx100_path = format!("{out_dir}/dx100.wav");
        save_wav(&dx100_path, &dx100_samples, dx100_sr, 1.0);

        println!("[synth]  → {synth_path}  (gain {gain_db:+.1} dB applied for level match)");
        print_stats("synth", &AudioStats {
            rms:          synth.rms * gain,
            peak:         synth.peak * gain,
            sustain_rms:  synth.sustain_rms * gain,
            crest_factor: synth.crest_factor,  // unchanged by linear gain
        });
        println!("[dx100]  → {dx100_path}");
        print_stats("dx100", &dx100);

        println!("\n── Comparison (after level match) ──────────────────────────────");
        let cf_diff = dx100.crest_factor - synth.crest_factor;
        println!("  crest-factor dx100-synth : {:+.4}  (>0 = dx100 more harmonic than synth)", cf_diff);
        if dx100.peak < 0.01 {
            println!("  ⚠  DX100 peak={:.4} — recording level very low; check hardware volume / input gain", dx100.peak);
        }
    } else {
        let synth_path = format!("{out_dir}/synth.wav");
        save_wav(&synth_path, &synth_samples, 44100, 1.0);
        println!("[synth]  → {synth_path}");
        print_stats("synth", &synth);
        if midi_out.is_none() { println!("\n(skip DX100 recording: --midi-out not specified)"); }
        if audio_in.is_none() { println!("(skip DX100 recording: --audio-in not specified)"); }
        println!("Run with --list to see available devices.");
    }
}

fn save_wav(path: &str, samples: &[f32], sr: u32, gain: f32) {
    let spec = hound::WavSpec {
        channels: 1, sample_rate: sr,
        bits_per_sample: 16, sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .unwrap_or_else(|e| panic!("Cannot create {path}: {e}"));
    for &s in samples {
        let v = ((s * gain).clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(v).unwrap();
    }
    writer.finalize().unwrap();
}

struct AudioStats {
    rms:          f32,   // full-window RMS (hold + release)
    peak:         f32,   // peak absolute sample
    sustain_rms:  f32,   // RMS of middle 50% of hold window (steady state)
    crest_factor: f32,   // sustain_rms / peak  (sine wave ≈ 0.707)
}

fn print_stats(label: &str, s: &AudioStats) {
    println!("  {label}  rms={:.4}  peak={:.4}  sustain_rms={:.4}  crest={:.4}",
        s.rms, s.peak, s.sustain_rms, s.crest_factor);
}

// ── Softsynth render ──────────────────────────────────────────────────────────

fn render_synth(
    voice: &xdx_core::dx100::Dx100Voice,
    midi_note: u8,
    duration_s: f32,
    release_s: f32,
) -> (AudioStats, Vec<f32>) {
    const SR: u32 = 44100;
    let total    = ((duration_s + release_s) * SR as f32) as usize;
    let note_off = (duration_s * SR as f32) as usize;
    let sus_start = (duration_s * 0.25 * SR as f32) as usize;
    let sus_end   = (duration_s * 0.75 * SR as f32) as usize;

    let mut engine = FmEngine::new(SR as f32);
    engine.set_voice(voice.clone());
    engine.note_on(midi_note, 100);

    let mut samples = Vec::with_capacity(total);
    let mut buf     = vec![0.0f32; 512];
    let mut pos     = 0usize;
    let mut sum_sq  = 0.0f64;
    let mut sus_sq  = 0.0f64;
    let mut sus_n   = 0usize;
    let mut peak    = 0.0f32;

    while pos < total {
        let chunk = buf.len().min(total - pos);
        engine.render(&mut buf[..chunk]);
        if pos < note_off && pos + chunk >= note_off {
            engine.note_off(midi_note);
        }
        for (j, &s) in buf[..chunk].iter().enumerate() {
            let i = pos + j;
            sum_sq += (s as f64).powi(2);
            if s.abs() > peak { peak = s.abs(); }
            if i >= sus_start && i < sus_end {
                sus_sq += (s as f64).powi(2);
                sus_n  += 1;
            }
            samples.push(s);
        }
        pos += chunk;
    }

    let rms          = (sum_sq / total as f64).sqrt() as f32;
    let sustain_rms  = if sus_n > 0 { (sus_sq / sus_n as f64).sqrt() as f32 } else { 0.0 };
    let crest_factor = if peak > 0.0 { sustain_rms / peak } else { 0.0 };
    (AudioStats { rms, peak, sustain_rms, crest_factor }, samples)
}

// ── DX100 recording ───────────────────────────────────────────────────────────

fn record_dx100(
    midi_note: u8,
    channel: u8,
    duration_s: f32,
    release_s: f32,
    midi_out_name: &str,
    audio_in_name: &str,
) -> (AudioStats, Vec<f32>, u32) {
    const PRE_DELAY_MS: u64 = 300;

    // ── Open audio input ──────────────────────────────────────────────────────
    let host = cpal::default_host();
    let device = find_input_device(&host, audio_in_name)
        .unwrap_or_else(|| panic!("Audio input device not found: \"{audio_in_name}\""));
    let config = device.default_input_config()
        .expect("no default input config");
    let sr     = config.sample_rate().0;
    let ch     = config.channels() as usize;

    println!("  audio in: \"{}\"  {}Hz  {}ch", device.name().unwrap_or_default(), sr, ch);

    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let buf_cb = buffer.clone();

    let stream = device.build_input_stream::<f32, _, _>(
        &cpal::StreamConfig {
            channels:    config.channels(),
            sample_rate: config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        },
        move |data: &[f32], _| {
            // Mix to mono and accumulate
            let mut buf = buf_cb.lock().unwrap();
            for frame in data.chunks(ch) {
                let mono: f32 = frame.iter().sum::<f32>() / ch as f32;
                buf.push(mono);
            }
        },
        |err| eprintln!("audio input error: {err}"),
        None,
    ).expect("build input stream");

    stream.play().expect("start input stream");

    // ── Open MIDI OUT ─────────────────────────────────────────────────────────
    let mut midi = MidiManager::new();
    midi.open_out(midi_out_name)
        .unwrap_or_else(|e| panic!("MIDI OUT open failed: {e}"));

    let note_on      = [0x90 | (channel - 1), midi_note, 100];
    let note_off_msg = [0x80 | (channel - 1), midi_note, 0];

    // ── Timing sequence ───────────────────────────────────────────────────────
    println!("  waiting {PRE_DELAY_MS}ms pre-delay…");
    std::thread::sleep(Duration::from_millis(PRE_DELAY_MS));

    println!("  Note On  (note {midi_note})");
    midi.send(&note_on).expect("send Note On");
    std::thread::sleep(Duration::from_secs_f32(duration_s));

    println!("  Note Off");
    midi.send(&note_off_msg).expect("send Note Off");
    std::thread::sleep(Duration::from_secs_f32(release_s));

    // ── Stop and save ─────────────────────────────────────────────────────────
    drop(stream);

    let samples  = buffer.lock().unwrap().clone();
    let total    = samples.len();

    // Sustain window: middle 50% of hold within the recording
    let note_on_sample   = (PRE_DELAY_MS as f32 / 1000.0 * sr as f32) as usize;
    let note_dur_samples = (duration_s * sr as f32) as usize;
    let sus_start = note_on_sample + note_dur_samples / 4;
    let sus_end   = note_on_sample + note_dur_samples * 3 / 4;

    let mut sum_sq = 0.0f64;
    let mut sus_sq = 0.0f64;
    let mut sus_n  = 0usize;
    let mut peak   = 0.0f32;
    for (i, &s) in samples.iter().enumerate() {
        sum_sq += (s as f64).powi(2);
        if s.abs() > peak { peak = s.abs(); }
        if i >= sus_start && i < sus_end {
            sus_sq += (s as f64).powi(2);
            sus_n  += 1;
        }
    }
    let rms         = (sum_sq / total as f64).sqrt() as f32;
    let sustain_rms = if sus_n > 0 { (sus_sq / sus_n as f64).sqrt() as f32 } else { 0.0 };
    let crest_factor = if peak > 0.0 { sustain_rms / peak } else { 0.0 };

    (AudioStats { rms, peak, sustain_rms, crest_factor }, samples, sr)
}

// ── Device listing ────────────────────────────────────────────────────────────

fn list_devices() {
    println!("=== MIDI OUT ports ===");
    for (i, name) in MidiManager::list_out_ports().iter().enumerate() {
        println!("  {i}: {name}");
    }

    println!("\n=== Audio INPUT devices ===");
    let host = cpal::default_host();
    if let Ok(devices) = host.input_devices() {
        for (i, d) in devices.enumerate() {
            let name = d.name().unwrap_or_else(|_| "(unknown)".to_string());
            let cfg  = d.default_input_config()
                .map(|c| format!("{}Hz {}ch", c.sample_rate().0, c.channels()))
                .unwrap_or_else(|_| "?".to_string());
            let marker = if i == 0 { " ← default" } else { "" };
            println!("  {i}: {name}  [{cfg}]{marker}");
        }
    }
}

fn find_input_device(host: &cpal::Host, name: &str) -> Option<cpal::Device> {
    if name.is_empty() {
        return host.default_input_device();
    }
    host.input_devices().ok()?
        .find(|d| d.name().ok().as_deref() == Some(name))
}

// ── CLI helpers ───────────────────────────────────────────────────────────────

fn flag_val(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
}
