use eframe::egui::{self, Grid, RichText};
use std::path::PathBuf;
use xdx_core::dx100::Dx100Voice;
use xdx_core::sysex::{dx100_decode_1voice, dx100_encode_1voice};

static IVORY_EBONY_SYX: &[u8] = include_bytes!("../../testdata/syx/IvoryEbony.syx");

// ── lookup tables (from original dx100ParamCtrl.c) ────────────────────────────
const FREQ_TBL: &[&str] = &[
    "0.50","0.71","0.78","0.87","1.00","1.41","1.57","1.73",
    "2.00","2.82","3.00","3.14","3.46","4.00","4.24","4.71",
    "5.00","5.19","5.65","6.00","6.28","6.92","7.00","7.07",
    "7.85","8.00","8.48","8.65","9.00","9.42","9.89","10.00",
    "10.38","10.99","11.00","11.30","12.00","12.11","12.56","12.72",
    "13.00","13.84","14.00","14.10","14.13","15.00","15.55","15.57",
    "15.70","16.96","17.27","17.30","18.37","18.84","19.03","19.78",
    "20.41","20.76","21.20","21.98","22.49","23.55","24.22","25.95",
];
const DETUNE_TBL: &[&str] = &["-3","-2","-1","0","+1","+2","+3"];
const LFO_WAVE_TBL: &[&str] = &["SAW","SQU","TRI","S/H"];
const TRANSPOSE_TBL: &[&str] = &[
    "C 1","C#1","D 1","D#1","E 1","F 1","F#1","G 1","G#1","A 1","A#1","B 1",
    "C 2","C#2","D 2","D#2","E 2","F 2","F#2","G 2","G#2","A 2","A#2","B 2",
    "C 3","C#3","D 3","D#3","E 3","F 3","F#3","G 3","G#3","A 3","A#3","B 3",
    "C 4","C#4","D 4","D#4","E 4","F 4","F#4","G 4","G#4","A 4","A#4","B 4","C 5",
];
const PORTA_MODE_TBL: &[&str] = &["Full","Fing"];
const ON_OFF_TBL: &[&str] = &["OFF","ON"];
const POLY_MONO_TBL: &[&str] = &["POLY","MONO"];

fn tbl<'a>(t: &[&'a str], v: u8) -> &'a str {
    t.get(v as usize).copied().unwrap_or("?")
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("xdx - DX100/DX7 Editor")
            .with_inner_size([1100.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native("xdx", options, Box::new(|_cc| Ok(Box::new(App::new()))))
}

struct App {
    voice: Dx100Voice,
    file_path: Option<PathBuf>,
    status: String,
}

impl App {
    fn new() -> Self {
        Self {
            voice: dx100_decode_1voice(IVORY_EBONY_SYX).expect("decode failed"),
            file_path: None,
            status: "Test data loaded".to_string(),
        }
    }

    fn open_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SysEx", &["syx"])
            .pick_file()
        else { return };

        match std::fs::read(&path) {
            Err(e) => self.status = format!("Open failed: {e}"),
            Ok(bytes) => match dx100_decode_1voice(&bytes) {
                Err(e) => self.status = format!("Decode failed: {e:?}"),
                Ok(voice) => {
                    self.voice = voice;
                    self.status = format!("Opened: {}", path.display());
                    self.file_path = Some(path);
                }
            },
        }
    }

    fn save_file(&mut self) {
        let path = if let Some(p) = &self.file_path {
            // Already have a path — save directly (no dialog)
            p.clone()
        } else {
            // No path yet — ask where to save
            let Some(p) = rfd::FileDialog::new()
                .add_filter("SysEx", &["syx"])
                .set_file_name(format!("{}.syx", self.voice.name_str().trim()))
                .save_file()
            else { return };
            p
        };

        let bytes = dx100_encode_1voice(&self.voice, 0);
        match std::fs::write(&path, &bytes) {
            Err(e) => self.status = format!("Save failed: {e}"),
            Ok(()) => {
                self.status = format!("Saved: {}", path.display());
                self.file_path = Some(path);
            }
        }
    }

    fn save_as(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SysEx", &["syx"])
            .set_file_name(format!("{}.syx", self.voice.name_str().trim()))
            .save_file()
        else { return };

        let bytes = dx100_encode_1voice(&self.voice, 0);
        match std::fs::write(&path, &bytes) {
            Err(e) => self.status = format!("Save failed: {e}"),
            Ok(()) => {
                self.status = format!("Saved: {}", path.display());
                self.file_path = Some(path);
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Toolbar ──────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open").clicked() { self.open_file(); }
                if ui.button("Save").clicked() { self.save_file(); }
                if ui.button("Save As").clicked() { self.save_as(); }
                ui.separator();
                let filename = self.file_path.as_deref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("(test data)");
                ui.label(filename);
            });
        });

        // ── Status bar ───────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.label(&self.status);
        });

        // ── Main panel ───────────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                show_dx100_voice(ui, &self.voice);
            });
        });
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────
fn hdr(s: &str) -> RichText { RichText::new(s).strong().small() }
fn val(s: impl ToString) -> String { s.to_string() }

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).strong().small().color(egui::Color32::from_gray(160)));
}

// ── main layout ───────────────────────────────────────────────────────────────
fn show_dx100_voice(ui: &mut egui::Ui, v: &Dx100Voice) {
    let sp = [8.0_f32, 4.0_f32];

    // ── Row 0: PATCHNAME ─────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label(hdr("PATCHNAME"));
        ui.add(egui::Label::new(
            RichText::new(v.name_str()).monospace().size(14.0)
        ));
    });
    ui.add_space(4.0);

    // ── Row 1-4: Global + per-operator AME / EG BIAS / VELOCITY ─────────────
    // Display order: OP4 (top) -> OP3 -> OP2 -> OP1 (bottom), matching original
    ui.horizontal(|ui| {
        ui.add_space(120.0);
        section_label(ui, "-------------- LFO --------------");
        ui.add_space(30.0);
        section_label(ui, "-- MODULATION SENSITIVITY --");
        ui.add_space(18.0);
        section_label(ui, "-- KEY --");
    });

    Grid::new("global_hdr").num_columns(14).spacing(sp).show(ui, |ui| {
        for h in &["ALGORITHM","FEEDBACK","WAVE","SPEED","DELAY","PMD","AMD","SYNC",
                   "PITCH","AMPLITUDE","AME","EG BIAS","VELOCITY",""] {
            ui.label(hdr(h));
        }
        ui.end_row();

        // First row: global params + OP4 sensitivity
        ui.label(val(v.algorithm + 1));
        ui.label(val(v.feedback));
        ui.label(tbl(LFO_WAVE_TBL, v.lfo_wave));
        ui.label(val(v.lfo_speed));
        ui.label(val(v.lfo_delay));
        ui.label(val(v.lfo_pmd));
        ui.label(val(v.lfo_amd));
        ui.label(tbl(ON_OFF_TBL, v.lfo_sync));
        ui.label(val(v.pitch_mod_sens));
        ui.label(val(v.amp_mod_sens));
        ui.label(val(v.ops[3].amp_mod_en));
        ui.label(val(v.ops[3].eg_bias_sens));
        ui.label(val(v.ops[3].key_vel_sens));
        ui.label(hdr("OPERATOR4"));
        ui.end_row();

        // OP3, OP2, OP1 sensitivity (cols 0-9 empty)
        for (op_idx, label) in [(2usize, "OPERATOR3"), (1, "OPERATOR2"), (0, "OPERATOR1")] {
            for _ in 0..10 { ui.label(""); }
            ui.label(val(v.ops[op_idx].amp_mod_en));
            ui.label(val(v.ops[op_idx].eg_bias_sens));
            ui.label(val(v.ops[op_idx].key_vel_sens));
            ui.label(hdr(label));
            ui.end_row();
        }
    });

    ui.add_space(6.0);

    // ── Row 5-8: OSCILLATOR + ENVELOPE GENERATOR + KEY SCALING + PITCH EG ───
    // Display order: OP4 (top) -> OP3 -> OP2 -> OP1 (bottom), Pitch EG on OP1 row
    ui.horizontal(|ui| {
        ui.add_space(66.0);
        section_label(ui, "- OSCILLATOR -");
        ui.add_space(8.0);
        section_label(ui, "-------------- ENVELOPE GENERATOR --------------");
        ui.add_space(20.0);
        section_label(ui, "- OPERATOR -");
        ui.add_space(6.0);
        section_label(ui, "-- KEY SCALING --");
        ui.add_space(4.0);
        section_label(ui, "-------- PITCH ENVELOPE GENERATOR --------");
    });

    Grid::new("op_hdr").num_columns(17).spacing(sp).show(ui, |ui| {
        for h in &["","RATIO","DETUNE","AR","D1R","D1L","D2R","RR",
                   "OUT LEVEL","RATE","LEVEL","PR1","PL1","PR2","PL2","PR3","PL3"] {
            ui.label(hdr(h));
        }
        ui.end_row();

        // OP4 -> OP3 -> OP2 -> OP1
        for (op_idx, label) in [(3usize,"OPERATOR4"),(2,"OPERATOR3"),(1,"OPERATOR2"),(0,"OPERATOR1")] {
            let op = &v.ops[op_idx];
            ui.label(hdr(label));
            ui.label(tbl(FREQ_TBL, op.freq_ratio));
            ui.label(tbl(DETUNE_TBL, op.detune));
            ui.label(val(op.ar));
            ui.label(val(op.d1r));
            ui.label(val(op.d1l));
            ui.label(val(op.d2r));
            ui.label(val(op.rr));
            ui.label(val(op.out_level));
            ui.label(val(op.kbd_rate_scl));
            ui.label(val(op.kbd_lev_scl));
            // Pitch EG on OPERATOR1 row only
            if op_idx == 0 {
                ui.label(val(v.pitch_eg_rate[0]));
                ui.label(val(v.pitch_eg_level[0]));
                ui.label(val(v.pitch_eg_rate[1]));
                ui.label(val(v.pitch_eg_level[1]));
                ui.label(val(v.pitch_eg_rate[2]));
                ui.label(val(v.pitch_eg_level[2]));
            }
            ui.end_row();
        }
    });

    ui.add_space(6.0);

    // ── Row 9-10: Performance controls ───────────────────────────────────────
    ui.horizontal(|ui| {
        ui.add_space(60.0);
        section_label(ui, "PITCH BEND");
        ui.add_space(32.0);
        section_label(ui, "-------- PORTAMENTO --------");
        ui.add_space(16.0);
        section_label(ui, "---- FOOT CONTROL ----");
        ui.add_space(16.0);
        section_label(ui, "-- WHEEL RANGE --");
        ui.add_space(8.0);
        section_label(ui, "------ BREATH CONTROLLER RANGE ------");
    });

    Grid::new("perf_hdr").num_columns(16).spacing(sp).show(ui, |ui| {
        for h in &["POLY/MONO","RANGE","MODE","TIME","FOOT SW","VOLUME","SUSTAIN",
                   "PITCH","AMPLITUDE","PITCH","AMPLITUDE","PITCH BIAS","EG BIAS",
                   "CHORUS","TRANSPOSE",""] {
            ui.label(hdr(h));
        }
        ui.end_row();

        ui.label(tbl(POLY_MONO_TBL, v.poly_mono));
        ui.label(val(v.pb_range));
        ui.label(tbl(PORTA_MODE_TBL, v.porta_mode));
        ui.label(val(v.porta_time));
        ui.label(tbl(ON_OFF_TBL, v.portamento));
        ui.label(val(v.fc_volume));
        ui.label(tbl(ON_OFF_TBL, v.sustain));
        ui.label(val(v.mw_pitch));
        ui.label(val(v.mw_amplitude));
        ui.label(val(v.bc_pitch));
        ui.label(val(v.bc_amplitude));
        ui.label(val(v.bc_pitch_bias));
        ui.label(val(v.bc_eg_bias));
        ui.label(tbl(ON_OFF_TBL, v.chorus));
        ui.label(tbl(TRANSPOSE_TBL, v.transpose));
        ui.label("");
        ui.end_row();
    });
}
