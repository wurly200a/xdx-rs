use nih_plug::prelude::*;
use parking_lot::Mutex;
use std::sync::Arc;
use xdx_core::dx100::Dx100Voice;
use xdx_core::sysex::dx100_decode_1voice;
use xdx_synth::FmEngine;

struct XdxVst {
    params: Arc<XdxParams>,
    engine: FmEngine,
    sample_rate: f32,
    current_voice: Dx100Voice,
    /// Raw SysEx bytes most recently applied to the engine; used to detect changes in voice_sysex.
    applied_sysex: Vec<u8>,
}

#[derive(Default, Params)]
struct XdxParams {
    /// Current voice as a DX100 1-voice SysEx dump (101 bytes). Persisted in .vstpreset.
    #[persist = "voice_sysex"]
    voice_sysex: Mutex<Vec<u8>>,

    /// Absolute path to a .syx file (1-voice format). When non-empty, loaded on initialize().
    #[persist = "syx_path"]
    syx_path: Mutex<String>,
}

impl Default for XdxVst {
    fn default() -> Self {
        Self {
            params: Arc::new(XdxParams::default()),
            engine: FmEngine::new(44100.0),
            sample_rate: 44100.0,
            current_voice: Dx100Voice::default(),
            applied_sysex: Vec::new(),
        }
    }
}

impl Plugin for XdxVst {
    const NAME: &'static str = "XDX Synth";
    const VENDOR: &'static str = "xdx-rs";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.engine.reset_sample_rate(self.sample_rate);

        // If syx_path is set, load the file and overwrite voice_sysex.
        let path = self.params.syx_path.lock().clone();
        if !path.is_empty() {
            match std::fs::read(&path) {
                Ok(data) => *self.params.voice_sysex.lock() = data,
                Err(e) => nih_warn!("failed to load .syx '{}': {}", path, e),
            }
        }

        // Apply voice_sysex to the engine if non-empty.
        self.apply_voice_sysex();

        true
    }

    fn reset(&mut self) {
        self.engine.reset_sample_rate(self.sample_rate);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Poll voice_sysex for changes (e.g., DAW loaded a preset).
        let changed = {
            if let Some(guard) = self.params.voice_sysex.try_lock() {
                if *guard != self.applied_sysex {
                    Some(guard.as_slice().to_vec())
                } else {
                    None
                }
            } else {
                None
            }
        };
        if let Some(sysex) = changed {
            self.apply_sysex_bytes(sysex);
        }

        // Route MIDI note events to the engine.
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::NoteOn { note, velocity, .. } => {
                    self.engine.note_on(note, (velocity * 127.0).round() as u8);
                }
                NoteEvent::NoteOff { note, .. } => {
                    self.engine.note_off(note);
                }
                _ => {}
            }
        }

        // Render audio into stereo output channels.
        let output = buffer.as_slice();
        if let [left, right, ..] = output {
            self.engine.render_block(left, right);
        }

        ProcessStatus::Normal
    }
}

impl XdxVst {
    /// Decode voice_sysex from params and apply to the engine.
    fn apply_voice_sysex(&mut self) {
        let sysex = self.params.voice_sysex.lock().as_slice().to_vec();
        if !sysex.is_empty() {
            self.apply_sysex_bytes(sysex);
        }
    }

    /// Decode raw SysEx bytes, apply the voice to the engine, and update tracking state.
    fn apply_sysex_bytes(&mut self, sysex: Vec<u8>) {
        match dx100_decode_1voice(&sysex) {
            Ok(voice) => {
                self.engine.set_voice(voice.clone());
                self.current_voice = voice;
                self.applied_sysex = sysex;
            }
            Err(e) => nih_warn!("invalid voice sysex: {:?}", e),
        }
    }
}

impl Vst3Plugin for XdxVst {
    // 16-byte unique class ID — change before distributing
    const VST3_CLASS_ID: [u8; 16] = *b"XdxSynth12345678";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_vst3!(XdxVst);
