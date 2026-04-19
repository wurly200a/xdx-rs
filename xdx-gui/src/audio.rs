use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use xdx_core::dx100::Dx100Voice;
use xdx_synth::FmEngine;

pub struct AudioHandle {
    engine:  Arc<Mutex<FmEngine>>,
    _stream: cpal::Stream,
    pub sample_rate: f32,
}

impl AudioHandle {
    pub fn start() -> Result<Self, String> {
        let host   = cpal::default_host();
        let device = host.default_output_device()
            .ok_or("no default audio output device")?;
        let sup    = device.default_output_config()
            .map_err(|e| e.to_string())?;

        let sr       = sup.sample_rate().0 as f32;
        let channels = sup.channels() as usize;

        let engine    = Arc::new(Mutex::new(FmEngine::new(sr)));
        let engine_cb = engine.clone();

        // Request F32 samples; WASAPI / CoreAudio / ALSA all support this.
        let config = cpal::StreamConfig {
            channels:    sup.channels(),
            sample_rate: sup.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = device
            .build_output_stream::<f32, _, _>(
                &config,
                move |data: &mut [f32], _| {
                    let n_frames = data.len() / channels;
                    let mut mono = vec![0.0f32; n_frames];
                    if let Ok(mut eng) = engine_cb.try_lock() {
                        eng.render(&mut mono);
                    }
                    for (frame, &s) in data.chunks_mut(channels).zip(mono.iter()) {
                        frame.fill(s);
                    }
                },
                |err| eprintln!("audio error: {err}"),
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;

        Ok(Self { engine, _stream: stream, sample_rate: sr })
    }

    pub fn note_on(&self, note: u8, velocity: u8) {
        if let Ok(mut eng) = self.engine.lock() {
            eng.note_on(note, velocity);
        }
    }

    pub fn note_off(&self, note: u8) {
        if let Ok(mut eng) = self.engine.lock() {
            eng.note_off(note);
        }
    }

    pub fn set_voice(&self, voice: Dx100Voice) {
        if let Ok(mut eng) = self.engine.lock() {
            eng.set_voice(voice);
        }
    }
}
