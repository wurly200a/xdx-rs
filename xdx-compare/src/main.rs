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

    // ── 1. Render softsynth ───────────────────────────────────────────────────
    let synth_path = format!("{out_dir}/synth.wav");
    let synth_rms = render_synth(&voice, midi_note, duration, release, &synth_path);
    println!("\n[synth]  → {synth_path}  RMS={synth_rms:.4}");

    // ── 2. Record DX100 ───────────────────────────────────────────────────────
    if midi_out.is_some() && audio_in.is_some() {
        let dx100_path = format!("{out_dir}/dx100.wav");
        let dx100_rms = record_dx100(
            midi_note, channel, duration, release,
            midi_out.as_deref().unwrap(),
            audio_in.as_deref().unwrap(),
            &dx100_path,
        );
        println!("[dx100]  → {dx100_path}  RMS={dx100_rms:.4}");

        let ratio = if synth_rms > 0.0 { dx100_rms / synth_rms } else { 0.0 };
        println!("\nRMS ratio dx100/synth: {ratio:.3}");
    } else {
        if midi_out.is_none() { println!("\n(skip DX100 recording: --midi-out not specified)"); }
        if audio_in.is_none() { println!("(skip DX100 recording: --audio-in not specified)"); }
        println!("Run with --list to see available devices.");
    }
}

// ── Softsynth render ──────────────────────────────────────────────────────────

fn render_synth(
    voice: &xdx_core::dx100::Dx100Voice,
    midi_note: u8,
    duration_s: f32,
    release_s: f32,
    out_path: &str,
) -> f32 {
    const SR: u32 = 44100;
    let total   = ((duration_s + release_s) * SR as f32) as usize;
    let note_off = (duration_s * SR as f32) as usize;

    let mut engine = FmEngine::new(SR as f32);
    engine.set_voice(voice.clone());
    engine.note_on(midi_note, 100);

    let spec = hound::WavSpec {
        channels: 1, sample_rate: SR,
        bits_per_sample: 16, sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(out_path, spec)
        .unwrap_or_else(|e| panic!("Cannot create {out_path}: {e}"));

    let mut buf = vec![0.0f32; 512];
    let mut pos = 0usize;
    let mut sum_sq = 0.0f64;

    while pos < total {
        let chunk = buf.len().min(total - pos);
        engine.render(&mut buf[..chunk]);
        if pos < note_off && pos + chunk >= note_off {
            engine.note_off(midi_note);
        }
        for &s in &buf[..chunk] {
            sum_sq += (s as f64).powi(2);
            let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(v).unwrap();
        }
        pos += chunk;
    }
    writer.finalize().unwrap();
    (sum_sq / total as f64).sqrt() as f32
}

// ── DX100 recording ───────────────────────────────────────────────────────────

fn record_dx100(
    midi_note: u8,
    channel: u8,
    duration_s: f32,
    release_s: f32,
    midi_out_name: &str,
    audio_in_name: &str,
    out_path: &str,
) -> f32 {
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

    let note_on  = [0x90 | (channel - 1), midi_note, 100];
    let note_off = [0x80 | (channel - 1), midi_note, 0];

    // ── Timing sequence ───────────────────────────────────────────────────────
    println!("  waiting {PRE_DELAY_MS}ms pre-delay…");
    std::thread::sleep(Duration::from_millis(PRE_DELAY_MS));

    println!("  Note On  (note {midi_note})");
    midi.send(&note_on).expect("send Note On");
    std::thread::sleep(Duration::from_secs_f32(duration_s));

    println!("  Note Off");
    midi.send(&note_off).expect("send Note Off");
    std::thread::sleep(Duration::from_secs_f32(release_s));

    // ── Stop and save ─────────────────────────────────────────────────────────
    drop(stream);

    let samples = buffer.lock().unwrap().clone();
    let total   = samples.len();
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
    let rms = (sum_sq / total as f64).sqrt() as f32;

    let spec = hound::WavSpec {
        channels: 1, sample_rate: sr,
        bits_per_sample: 16, sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(out_path, spec)
        .unwrap_or_else(|e| panic!("Cannot create {out_path}: {e}"));
    for s in &samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(v).unwrap();
    }
    writer.finalize().unwrap();

    rms
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
