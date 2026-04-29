//! Batch-record all voices in a DX100 32-voice bank against hardware.
//!
//! Sends the bank SysEx once, then cycles through voices 0..(BANK_VOICES-1)
//! using Program Change, recording each voice to a separate WAV file.
//! The softsynth render of each voice is saved alongside for comparison.
//!
//! Usage:
//!   record-eg-bank --list
//!   record-eg-bank <bank.syx> --midi-out <port> --audio-in <device> [options]
//!
//! Options:
//!   --list              List available MIDI OUT and audio IN devices
//!   --midi-out <name>   MIDI OUT port connected to DX100
//!   --audio-in <name>   Audio IN device (exact name from --list)
//!   --note <n>          MIDI note to play (default: 69 = A4)
//!   --hold <secs>       Note hold duration per voice  (default: 3.0)
//!   --release <secs>    Release capture time per voice (default: 3.0)
//!   --channel <1-16>    MIDI channel (default: 1)
//!   --count <n>         Record only the first N voices (default: all)
//!   --out <dir>         Output directory (default: out/eg_bank)
//!
//! Output structure:
//!   <dir>/dx100/<NN>_<name>.wav  — DX100 hardware recording
//!   <dir>/synth/<NN>_<name>.wav  — softsynth render (for compare_attack)

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use xdx_core::dx100::BANK_VOICES;
use xdx_core::sysex::{dx100_decode_32voice, dx100_encode_32voice};
use xdx_midi::MidiManager;
use xdx_synth::FmEngine;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--list") {
        list_devices();
        return;
    }

    let syx_path = args.get(1).unwrap_or_else(|| {
        eprintln!("Usage: record-eg-bank <bank.syx> --midi-out <port> --audio-in <device>");
        eprintln!("       record-eg-bank --list");
        std::process::exit(1);
    });

    let midi_out = flag_val(&args, "--midi-out");
    let audio_in = flag_val(&args, "--audio-in");
    let midi_note: u8 = flag_val(&args, "--note")
        .and_then(|s| s.parse().ok())
        .unwrap_or(69);
    let hold_s: f32 = flag_val(&args, "--hold")
        .and_then(|s| s.parse().ok())
        .unwrap_or(3.0);
    let release_s: f32 = flag_val(&args, "--release")
        .and_then(|s| s.parse().ok())
        .unwrap_or(3.0);
    let channel: u8 = flag_val(&args, "--channel")
        .and_then(|s| s.parse::<u8>().ok())
        .map(|c| c.clamp(1, 16))
        .unwrap_or(1);
    let out_dir = flag_val(&args, "--out").unwrap_or_else(|| "out/eg_bank".to_string());
    let count_limit = flag_val(&args, "--count").and_then(|s| s.parse::<usize>().ok());

    // Load bank
    let bytes = std::fs::read(syx_path).unwrap_or_else(|e| panic!("Cannot read {syx_path}: {e}"));
    let voices = dx100_decode_32voice(&bytes).unwrap_or_else(|e| panic!("Decode failed: {e:?}"));
    let n = voices
        .len()
        .min(BANK_VOICES)
        .min(count_limit.unwrap_or(usize::MAX));

    println!("=== record-eg-bank ===");
    println!("Bank:     {syx_path}  ({n} voices)");
    println!("Note:     {midi_note}  channel: {channel}");
    println!(
        "Timing:   {hold_s:.1}s hold + {release_s:.1}s release  (~{:.0}s total)",
        n as f32 * (0.3 + hold_s + release_s + 0.3)
    );
    println!("Output:   {out_dir}/");
    println!();

    std::fs::create_dir_all(format!("{out_dir}/dx100")).expect("create dx100 output dir");
    std::fs::create_dir_all(format!("{out_dir}/synth")).expect("create synth output dir");

    let do_record = midi_out.is_some() && audio_in.is_some();
    if !do_record {
        println!("(--midi-out or --audio-in not specified; only synth renders will be saved)\n");
    }

    // Open MIDI once and send bank SysEx
    let mut midi: Option<MidiManager> = if do_record {
        let mut m = MidiManager::new();
        m.open_out(midi_out.as_deref().unwrap())
            .unwrap_or_else(|e| panic!("MIDI OUT open failed: {e}"));
        let bank_syx = dx100_encode_32voice(&voices, channel - 1);
        m.send(&bank_syx).expect("send bank SysEx");
        println!(
            "Sent bank SysEx ({} bytes). Waiting 600ms for DX100 to load…",
            bank_syx.len()
        );
        std::thread::sleep(Duration::from_millis(600));
        println!();
        Some(m)
    } else {
        None
    };

    let est_per_voice = 0.3 + hold_s + release_s + 0.3;

    for i in 0..n {
        let voice = &voices[i];
        let name = voice.name_str();
        let safe_name: String = name
            .trim()
            .chars()
            .map(|c| match c {
                ' ' => '_',
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
                c => c,
            })
            .collect();
        let tag = format!("{:02}_{}", i + 1, safe_name);

        println!(
            "[{:2}/{}]  {:<12}  (est. {:.0}s remaining)",
            i + 1,
            n,
            name,
            (n - i) as f32 * est_per_voice
        );

        // Always render synth
        let synth_samples = render_synth(voice, midi_note, hold_s, release_s);
        let synth_path = format!("{out_dir}/synth/{tag}.wav");
        save_wav(&synth_path, &synth_samples, 44100);
        println!("         synth → {synth_path}");

        if let Some(ref mut m) = midi {
            // Program Change: select voice i
            m.send(&[0xC0 | (channel - 1), i as u8])
                .expect("send Program Change");
            // Settle time for DX100 to switch voices
            std::thread::sleep(Duration::from_millis(300));

            let (dx100_samples, sr) = record_voice(
                m,
                audio_in.as_deref().unwrap(),
                midi_note,
                channel,
                hold_s,
                release_s,
            );
            let dx100_path = format!("{out_dir}/dx100/{tag}.wav");
            save_wav(&dx100_path, &dx100_samples, sr);
            println!("         dx100 → {dx100_path}");

            // Brief pause before next Program Change
            std::thread::sleep(Duration::from_millis(300));
        }
        println!();
    }

    println!("Done. {} voices processed.", n);
}

// ── Recording ─────────────────────────────────────────────────────────────────

fn record_voice(
    midi: &mut MidiManager,
    audio_in_name: &str,
    midi_note: u8,
    channel: u8,
    hold_s: f32,
    release_s: f32,
) -> (Vec<f32>, u32) {
    const PRE_DELAY_MS: u64 = 300;

    let host = cpal::default_host();
    let device = find_input_device(&host, audio_in_name)
        .unwrap_or_else(|| panic!("Audio input not found: \"{audio_in_name}\""));
    let config = device
        .default_input_config()
        .expect("no default input config");
    let sr = config.sample_rate().0;
    let ch = config.channels() as usize;

    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let buf_cb = buffer.clone();

    let stream = device
        .build_input_stream::<f32, _, _>(
            &cpal::StreamConfig {
                channels: config.channels(),
                sample_rate: config.sample_rate(),
                buffer_size: cpal::BufferSize::Default,
            },
            move |data: &[f32], _| {
                let mut buf = buf_cb.lock().unwrap();
                for frame in data.chunks(ch) {
                    let mono: f32 = frame.iter().sum::<f32>() / ch as f32;
                    buf.push(mono);
                }
            },
            |err| eprintln!("audio input error: {err}"),
            None,
        )
        .expect("build input stream");
    stream.play().expect("start stream");

    std::thread::sleep(Duration::from_millis(PRE_DELAY_MS));
    midi.send(&[0x90 | (channel - 1), midi_note, 100])
        .expect("Note On");
    std::thread::sleep(Duration::from_secs_f32(hold_s));
    midi.send(&[0x80 | (channel - 1), midi_note, 0])
        .expect("Note Off");
    std::thread::sleep(Duration::from_secs_f32(release_s));

    drop(stream);
    let samples = buffer.lock().unwrap().clone();
    (samples, sr)
}

// ── Softsynth render ──────────────────────────────────────────────────────────

fn render_synth(
    voice: &xdx_core::dx100::Dx100Voice,
    midi_note: u8,
    hold_s: f32,
    release_s: f32,
) -> Vec<f32> {
    const SR: u32 = 44100;
    let total = ((hold_s + release_s) * SR as f32) as usize;
    let note_off_pos = (hold_s * SR as f32) as usize;

    let mut engine = FmEngine::new(SR as f32);
    engine.set_voice(voice.clone());
    engine.note_on(midi_note, 100);

    let mut samples = Vec::with_capacity(total);
    let mut buf = vec![0.0f32; 512];
    let mut pos = 0usize;

    while pos < total {
        let chunk = buf.len().min(total - pos);
        engine.render(&mut buf[..chunk]);
        if pos < note_off_pos && pos + chunk >= note_off_pos {
            engine.note_off(midi_note);
        }
        samples.extend_from_slice(&buf[..chunk]);
        pos += chunk;
    }
    samples
}

// ── WAV output ────────────────────────────────────────────────────────────────

fn save_wav(path: &str, samples: &[f32], sr: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sr,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .unwrap_or_else(|e| panic!("Cannot create {path}: {e}"));
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(v).unwrap();
    }
    writer.finalize().unwrap();
}

// ── Device helpers ────────────────────────────────────────────────────────────

fn find_input_device(host: &cpal::Host, name: &str) -> Option<cpal::Device> {
    if name.is_empty() {
        return host.default_input_device();
    }
    host.input_devices()
        .ok()?
        .find(|d| d.name().ok().as_deref() == Some(name))
}

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
            let cfg = d
                .default_input_config()
                .map(|c| format!("{}Hz {}ch", c.sample_rate().0, c.channels()))
                .unwrap_or_else(|_| "?".to_string());
            println!("  {i}: {name}  [{cfg}]");
        }
    }
}

// ── CLI helper ────────────────────────────────────────────────────────────────

fn flag_val(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}
