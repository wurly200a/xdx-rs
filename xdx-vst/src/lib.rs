use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
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

#[derive(Params)]
struct XdxParams {
    /// Editor window state (size). Persisted so the window reopens at the same size.
    #[persist = "editor"]
    editor_state: Arc<EguiState>,

    /// Current voice as a DX100 1-voice SysEx dump (101 bytes). Persisted in .vstpreset.
    #[persist = "voice_sysex"]
    voice_sysex: Mutex<Vec<u8>>,

    /// Absolute path to the last loaded .syx file. Reloaded on initialize() if set.
    #[persist = "syx_path"]
    syx_path: Mutex<String>,
}

impl Default for XdxParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(320, 64),
            voice_sysex: Mutex::new(Vec::new()),
            syx_path: Mutex::new(String::new()),
        }
    }
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

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        create_egui_editor(
            self.params.editor_state.clone(),
            params,
            |_ctx, _params| {}, // build: one-time init (no-op)
            |ctx, _setter, params| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Load SysEx").clicked() {
                            let params = params.clone();
                            std::thread::spawn(move || {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("SysEx", &["syx"])
                                    .pick_file()
                                {
                                    match std::fs::read(&path) {
                                        Ok(data) => {
                                            *params.syx_path.lock() =
                                                path.to_string_lossy().into_owned();
                                            *params.voice_sysex.lock() = data;
                                        }
                                        Err(e) => {
                                            nih_error!("failed to read {}: {}", path.display(), e);
                                        }
                                    }
                                }
                            });
                        }

                        let name = voice_name_from_sysex(&params.voice_sysex.lock());
                        ui.label(name);
                    });
                });
            },
        )
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
        // Poll voice_sysex for changes (e.g., DAW loaded a preset, or GUI loaded a file).
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
    fn apply_voice_sysex(&mut self) {
        let sysex = self.params.voice_sysex.lock().as_slice().to_vec();
        if !sysex.is_empty() {
            self.apply_sysex_bytes(sysex);
        }
    }

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

/// Extract the voice name from a 101-byte DX100 1-voice SysEx dump.
/// Name occupies payload bytes 77..87, i.e., raw bytes 83..93.
fn voice_name_from_sysex(sysex: &[u8]) -> String {
    if sysex.len() == 101 {
        std::str::from_utf8(&sysex[83..93])
            .unwrap_or("????????")
            .trim_end()
            .to_string()
    } else {
        "INIT".to_string()
    }
}

impl Vst3Plugin for XdxVst {
    // 16-byte unique class ID — change before distributing
    const VST3_CLASS_ID: [u8; 16] = *b"XdxSynth12345678";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_vst3!(XdxVst);
