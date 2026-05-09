use nih_plug::prelude::*;
use std::sync::Arc;
use xdx_synth::FmEngine;

struct XdxVst {
    params: Arc<XdxParams>,
    engine: FmEngine,
    sample_rate: f32,
}

#[derive(Default, Params)]
struct XdxParams {}

impl Default for XdxVst {
    fn default() -> Self {
        Self {
            params: Arc::new(XdxParams::default()),
            engine: FmEngine::new(44100.0),
            sample_rate: 44100.0,
        }
    }
}

impl Plugin for XdxVst {
    const NAME: &'static str = "XDX Synth";
    const VENDOR: &'static str = "xdx-rs";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
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
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::NoteOn { note, velocity, .. } => {
                    let vel_u8 = (velocity * 127.0).round() as u8;
                    self.engine.note_on(note, vel_u8);
                }
                NoteEvent::NoteOff { note, .. } => {
                    self.engine.note_off(note);
                }
                _ => {}
            }
        }

        // Split the stereo output buffer and render mono-summed audio into both channels.
        let output = buffer.as_slice();
        if let [left, right, ..] = output {
            self.engine.render_block(left, right);
        }

        ProcessStatus::Normal
    }
}

impl Vst3Plugin for XdxVst {
    // 16-byte unique class ID — change before distributing
    const VST3_CLASS_ID: [u8; 16] = *b"XdxSynth12345678";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_vst3!(XdxVst);
