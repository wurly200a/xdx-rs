use eframe::egui;
use xdx_core::dx100::Dx100Voice;
use xdx_core::sysex::dx100_decode_1voice;

static IVORY_EBONY_SYX: &[u8] = include_bytes!("../../testdata/syx/IvoryEbony.syx");

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("xdx - DX100/DX7 Editor")
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "xdx",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}

struct App {
    voice: Dx100Voice,
}

impl App {
    fn new() -> Self {
        let voice = dx100_decode_1voice(IVORY_EBONY_SYX)
            .expect("failed to decode test SysEx");
        Self { voice }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            show_dx100_voice(ui, &self.voice);
        });
    }
}

fn show_dx100_voice(ui: &mut egui::Ui, v: &Dx100Voice) {
    ui.heading(format!("Voice: {}", v.name_str()));
    ui.separator();

    // ── Global params ──────────────────────────────────────────────
    egui::Grid::new("global")
        .num_columns(4)
        .spacing([20.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Algorithm");
            ui.label(format!("{}", v.algorithm + 1)); // display 1-8
            ui.label("Feedback");
            ui.label(format!("{}", v.feedback));
            ui.end_row();

            ui.label("LFO Speed");
            ui.label(format!("{}", v.lfo_speed));
            ui.label("LFO Delay");
            ui.label(format!("{}", v.lfo_delay));
            ui.end_row();

            ui.label("LFO Wave");
            ui.label(lfo_wave_name(v.lfo_wave));
            ui.label("LFO Sync");
            ui.label(format!("{}", v.lfo_sync));
            ui.end_row();

            ui.label("PMD");
            ui.label(format!("{}", v.lfo_pmd));
            ui.label("AMD");
            ui.label(format!("{}", v.lfo_amd));
            ui.end_row();

            ui.label("Pitch Mod Sens");
            ui.label(format!("{}", v.pitch_mod_sens));
            ui.label("Amp Mod Sens");
            ui.label(format!("{}", v.amp_mod_sens));
            ui.end_row();

            ui.label("Transpose");
            ui.label(format!("{:+}", v.transpose as i8 - 24));
            ui.label("Poly/Mono");
            ui.label(if v.poly_mono == 0 { "Poly" } else { "Mono" });
            ui.end_row();
        });

    ui.separator();

    // ── Pitch EG ───────────────────────────────────────────────────
    ui.label("Pitch EG");
    egui::Grid::new("pitch_eg")
        .num_columns(7)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label("");
            ui.label("R1"); ui.label("R2"); ui.label("R3");
            ui.label("L1"); ui.label("L2"); ui.label("L3");
            ui.end_row();
            ui.label("");
            for r in &v.pitch_eg_rate  { ui.label(format!("{}", r)); }
            for l in &v.pitch_eg_level { ui.label(format!("{}", l)); }
            ui.end_row();
        });

    ui.separator();

    // ── Operators ──────────────────────────────────────────────────
    ui.label("Operators");
    egui::Grid::new("operators")
        .num_columns(14)
        .spacing([10.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            // header
            for h in &["OP", "AR", "D1R", "D2R", "RR", "D1L",
                       "KLS", "KRS", "EGB", "AME", "VEL", "OL", "FREQ", "DET"] {
                ui.label(egui::RichText::new(*h).strong());
            }
            ui.end_row();

            // OP1..OP4
            for (i, op) in v.ops.iter().enumerate() {
                ui.label(format!("OP{}", i + 1));
                ui.label(format!("{}", op.ar));
                ui.label(format!("{}", op.d1r));
                ui.label(format!("{}", op.d2r));
                ui.label(format!("{}", op.rr));
                ui.label(format!("{}", op.d1l));
                ui.label(format!("{}", op.kbd_lev_scl));
                ui.label(format!("{}", op.kbd_rate_scl));
                ui.label(format!("{}", op.eg_bias_sens));
                ui.label(format!("{}", op.amp_mod_en));
                ui.label(format!("{}", op.key_vel_sens));
                ui.label(format!("{}", op.out_level));
                ui.label(format!("{}", op.freq_ratio));
                ui.label(format!("{:+}", op.detune as i8 - 3));
                ui.end_row();
            }
        });
}

fn lfo_wave_name(wave: u8) -> &'static str {
    match wave {
        0 => "Triangle",
        1 => "Saw Down",
        2 => "Saw Up",
        3 => "Square",
        _ => "?",
    }
}
