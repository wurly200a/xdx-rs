use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use xdx_core::dx100::Dx100Voice;
use xdx_synth::FmEngine;

enum AudioCmd {
    NoteOn(u8, u8),
    NoteOff(u8),
    SetVoice(Box<Dx100Voice>),
}

pub struct AudioHandle {
    tx: mpsc::Sender<AudioCmd>,
    _stream: cpal::Stream,
}

impl AudioHandle {
    pub fn start() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("no default audio output device")?;
        let sup = device.default_output_config().map_err(|e| e.to_string())?;

        let sr = sup.sample_rate().0 as f32;
        let channels = sup.channels() as usize;

        let (tx, rx) = mpsc::channel::<AudioCmd>();
        let mut engine = FmEngine::new(sr);

        let config = cpal::StreamConfig {
            channels: sup.channels(),
            sample_rate: sup.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = device
            .build_output_stream::<f32, _, _>(
                &config,
                move |data: &mut [f32], _| {
                    // Apply all pending GUI commands before rendering.
                    // try_recv() is non-blocking — no mutex, no stall.
                    while let Ok(cmd) = rx.try_recv() {
                        match cmd {
                            AudioCmd::NoteOn(note, vel) => engine.note_on(note, vel),
                            AudioCmd::NoteOff(note) => engine.note_off(note),
                            AudioCmd::SetVoice(voice) => engine.set_voice(*voice),
                        }
                    }
                    let n_frames = data.len() / channels;
                    let mut mono = vec![0.0f32; n_frames];
                    engine.render(&mut mono);
                    for (frame, &s) in data.chunks_mut(channels).zip(mono.iter()) {
                        frame.fill(s);
                    }
                },
                |err| eprintln!("audio error: {err}"),
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;

        Ok(Self {
            tx,
            _stream: stream,
        })
    }

    pub fn note_on(&self, note: u8, velocity: u8) {
        let _ = self.tx.send(AudioCmd::NoteOn(note, velocity));
    }

    pub fn note_off(&self, note: u8) {
        let _ = self.tx.send(AudioCmd::NoteOff(note));
    }

    pub fn set_voice(&self, voice: Dx100Voice) {
        let _ = self.tx.send(AudioCmd::SetVoice(Box::new(voice)));
    }
}
