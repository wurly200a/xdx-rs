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
const DETUNE_TBL:    &[&str] = &["-3","-2","-1","0","+1","+2","+3"];
const LFO_WAVE_TBL:  &[&str] = &["SAW","SQU","TRI","S/H"];
const ALGO_TBL:      &[&str] = &["1","2","3","4","5","6","7","8"];
const TRANSPOSE_TBL: &[&str] = &[
    "C 1","C#1","D 1","D#1","E 1","F 1","F#1","G 1","G#1","A 1","A#1","B 1",
    "C 2","C#2","D 2","D#2","E 2","F 2","F#2","G 2","G#2","A 2","A#2","B 2",
    "C 3","C#3","D 3","D#3","E 3","F 3","F#3","G 3","G#3","A 3","A#3","B 3",
    "C 4","C#4","D 4","D#4","E 4","F 4","F#4","G 4","G#4","A 4","A#4","B 4","C 5",
];
const PORTA_MODE_TBL: &[&str] = &["Full","Fing"];
const ON_OFF_TBL:     &[&str] = &["OFF","ON"];
const POLY_MONO_TBL:  &[&str] = &["POLY","MONO"];

// ── widget helpers ────────────────────────────────────────────────────────────

/// DragValue for a u8 parameter with explicit range.
fn dv(ui: &mut egui::Ui, val: &mut u8, min: u8, max: u8) {
    ui.add(egui::DragValue::new(val).range(min..=max));
}

/// ComboBox backed by a &[&str] lookup table.
fn cb(ui: &mut egui::Ui, id: impl std::hash::Hash, tbl: &[&str], val: &mut u8) {
    let selected = tbl.get(*val as usize).copied().unwrap_or("?");
    egui::ComboBox::from_id_source(id)
        .selected_text(selected)
        .width(52.0)
        .show_ui(ui, |ui| {
            for (i, &label) in tbl.iter().enumerate() {
                ui.selectable_value(val, i as u8, label);
            }
        });
}

/// Checkbox for a 0/1 u8 parameter (no label).
fn chk(ui: &mut egui::Ui, val: &mut u8) {
    let mut b = *val != 0;
    if ui.checkbox(&mut b, "").changed() {
        *val = b as u8;
    }
}

fn hdr(s: &str) -> RichText { RichText::new(s).strong().small() }

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).strong().small().color(egui::Color32::from_gray(160)));
}

// ── Tab state enums ───────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum SynthType { Dx100, Dx7 }

#[derive(PartialEq)]
enum VoiceMode { OneVoice, ThirtyTwo }

#[derive(PartialEq)]
enum ActiveTab { File, Synth }

// ── App ───────────────────────────────────────────────────────────────────────

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("xdx - DX100/DX7 Editor")
            .with_inner_size([1150.0, 580.0]),
        ..Default::default()
    };
    eframe::run_native("xdx", options, Box::new(|_cc| Ok(Box::new(App::new()))))
}

struct App {
    // tab state
    synth_type: SynthType,
    voice_mode: VoiceMode,
    active_tab: ActiveTab,
    // voice data
    voice:      Dx100Voice,
    name_buf:   String,          // TextEdit buffer for voice name
    file_path:  Option<PathBuf>,
    status:     String,
}

impl App {
    fn new() -> Self {
        let voice = dx100_decode_1voice(IVORY_EBONY_SYX).expect("decode failed");
        let name_buf = voice.name_str();
        Self {
            synth_type: SynthType::Dx100,
            voice_mode: VoiceMode::OneVoice,
            active_tab: ActiveTab::File,
            voice,
            name_buf,
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
                    self.name_buf = voice.name_str();
                    self.voice = voice;
                    self.status = format!("Opened: {}", path.display());
                    self.file_path = Some(path);
                }
            },
        }
    }

    fn save_file(&mut self) {
        let path = if let Some(p) = &self.file_path {
            p.clone()
        } else {
            let Some(p) = rfd::FileDialog::new()
                .add_filter("SysEx", &["syx"])
                .set_file_name(format!("{}.syx", self.voice.name_str().trim()))
                .save_file()
            else { return };
            p
        };
        self.write_file(path);
    }

    fn save_as(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SysEx", &["syx"])
            .set_file_name(format!("{}.syx", self.voice.name_str().trim()))
            .save_file()
        else { return };
        self.write_file(path);
    }

    fn write_file(&mut self, path: PathBuf) {
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
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            // Row 1: synth type
            ui.horizontal(|ui| {
                ui.label(hdr("SYNTH:"));
                ui.selectable_value(&mut self.synth_type, SynthType::Dx100, "DX100");
                ui.selectable_value(&mut self.synth_type, SynthType::Dx7,   "DX7");
            });
            ui.separator();
            // Row 2: voice mode
            ui.horizontal(|ui| {
                ui.label(hdr("MODE:"));
                ui.selectable_value(&mut self.voice_mode, VoiceMode::OneVoice, "1 VOICE");
                ui.selectable_value(&mut self.voice_mode, VoiceMode::ThirtyTwo, "32 VOICES");
            });
            ui.separator();
            // Row 3: function tabs + tab-specific controls
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, ActiveTab::File,  "File");
                ui.selectable_value(&mut self.active_tab, ActiveTab::Synth, "Synth");
                ui.separator();
                match self.active_tab {
                    ActiveTab::File => {
                        if ui.button("Open").clicked()    { self.open_file(); }
                        if ui.button("Save").clicked()    { self.save_file(); }
                        if ui.button("Save As").clicked() { self.save_as(); }
                        ui.separator();
                        let filename = self.file_path.as_deref()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .unwrap_or("(test data)");
                        ui.label(filename);
                    }
                    ActiveTab::Synth => {
                        ui.add_enabled(false, egui::Button::new("GET"));
                        ui.add_enabled(false, egui::Button::new("SET"));
                        ui.separator();
                        ui.label(egui::RichText::new("MIDI not yet connected").weak());
                    }
                }
            });
        });

        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.label(&self.status);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                show_dx100_voice(ui, &mut self.voice, &mut self.name_buf);
            });
        });
    }
}

// ── main panel ────────────────────────────────────────────────────────────────

fn show_dx100_voice(ui: &mut egui::Ui, v: &mut Dx100Voice, name_buf: &mut String) {
    let sp = [8.0_f32, 4.0_f32];

    // ── PATCHNAME ─────────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label(hdr("PATCHNAME"));
        let resp = ui.add(
            egui::TextEdit::singleline(name_buf)
                .desired_width(88.0)
                .font(egui::TextStyle::Monospace)
        );
        if resp.changed() {
            name_buf.truncate(10);
            for (i, b) in v.name.iter_mut().enumerate() {
                *b = name_buf.as_bytes().get(i).copied().unwrap_or(b' ');
            }
        }
    });
    ui.add_space(4.0);

    // ── Global + per-operator AME / EG BIAS / VELOCITY ───────────────────────
    ui.horizontal(|ui| {
        ui.add_space(120.0);
        section_label(ui, "-------------- LFO --------------");
        ui.add_space(30.0);
        section_label(ui, "-- MODULATION SENSITIVITY --");
        ui.add_space(18.0);
        section_label(ui, "-- KEY --");
    });

    Grid::new("global").num_columns(14).spacing(sp).show(ui, |ui| {
        for h in &["ALGORITHM","FEEDBACK","WAVE","SPEED","DELAY","PMD","AMD","SYNC",
                   "PITCH","AMPLITUDE","AME","EG BIAS","VELOCITY",""] {
            ui.label(hdr(h));
        }
        ui.end_row();

        // global values + OP4 sensitivity
        cb(ui, "algo",    ALGO_TBL,     &mut v.algorithm);
        dv(ui, &mut v.feedback,         0, 7);
        cb(ui, "lfowave", LFO_WAVE_TBL, &mut v.lfo_wave);
        dv(ui, &mut v.lfo_speed,        0, 99);
        dv(ui, &mut v.lfo_delay,        0, 99);
        dv(ui, &mut v.lfo_pmd,          0, 99);
        dv(ui, &mut v.lfo_amd,          0, 99);
        chk(ui, &mut v.lfo_sync);
        dv(ui, &mut v.pitch_mod_sens,   0, 7);
        dv(ui, &mut v.amp_mod_sens,     0, 3);
        chk(ui, &mut v.ops[3].amp_mod_en);
        dv(ui, &mut v.ops[3].eg_bias_sens, 0, 7);
        dv(ui, &mut v.ops[3].key_vel_sens, 0, 7);
        ui.label(hdr("OPERATOR4"));
        ui.end_row();

        // OP3, OP2, OP1 sensitivity
        for (op_idx, label) in [(2usize,"OPERATOR3"),(1,"OPERATOR2"),(0,"OPERATOR1")] {
            for _ in 0..10 { ui.label(""); }
            chk(ui, &mut v.ops[op_idx].amp_mod_en);
            dv(ui, &mut v.ops[op_idx].eg_bias_sens, 0, 7);
            dv(ui, &mut v.ops[op_idx].key_vel_sens, 0, 7);
            ui.label(hdr(label));
            ui.end_row();
        }
    });

    ui.add_space(6.0);

    // ── OSCILLATOR + EG + KEY SCALING + PITCH EG ─────────────────────────────
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

    Grid::new("operators").num_columns(17).spacing(sp).show(ui, |ui| {
        for h in &["","RATIO","DETUNE","AR","D1R","D1L","D2R","RR",
                   "OUT LEVEL","RATE","LEVEL","PR1","PL1","PR2","PL2","PR3","PL3"] {
            ui.label(hdr(h));
        }
        ui.end_row();

        for (op_idx, label) in [(3usize,"OPERATOR4"),(2,"OPERATOR3"),(1,"OPERATOR2"),(0,"OPERATOR1")] {
            let op = &mut v.ops[op_idx];
            ui.label(hdr(label));
            cb(ui, ("freq",  op_idx), FREQ_TBL,   &mut op.freq_ratio);
            cb(ui, ("det",   op_idx), DETUNE_TBL, &mut op.detune);
            dv(ui, &mut op.ar,           0, 31);
            dv(ui, &mut op.d1r,          0, 31);
            dv(ui, &mut op.d1l,          0, 15);
            dv(ui, &mut op.d2r,          0, 31);
            dv(ui, &mut op.rr,           0, 15);
            dv(ui, &mut op.out_level,    0, 99);
            dv(ui, &mut op.kbd_rate_scl, 0,  3);
            dv(ui, &mut op.kbd_lev_scl,  0, 99);
            // Pitch EG on OPERATOR1 row only
            if op_idx == 0 {
                dv(ui, &mut v.pitch_eg_rate[0],  0, 99);
                dv(ui, &mut v.pitch_eg_level[0], 0, 99);
                dv(ui, &mut v.pitch_eg_rate[1],  0, 99);
                dv(ui, &mut v.pitch_eg_level[1], 0, 99);
                dv(ui, &mut v.pitch_eg_rate[2],  0, 99);
                dv(ui, &mut v.pitch_eg_level[2], 0, 99);
            }
            ui.end_row();
        }
    });

    ui.add_space(6.0);

    // ── Performance controls ──────────────────────────────────────────────────
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

    Grid::new("perf").num_columns(15).spacing(sp).show(ui, |ui| {
        for h in &["POLY/MONO","RANGE","MODE","TIME","FOOT SW","VOLUME","SUSTAIN",
                   "PITCH","AMPLITUDE","PITCH","AMPLITUDE","PITCH BIAS","EG BIAS",
                   "CHORUS","TRANSPOSE"] {
            ui.label(hdr(h));
        }
        ui.end_row();

        cb(ui, "polymono",   POLY_MONO_TBL,  &mut v.poly_mono);
        dv(ui, &mut v.pb_range,              0, 12);
        cb(ui, "portamode",  PORTA_MODE_TBL, &mut v.porta_mode);
        dv(ui, &mut v.porta_time,            0, 99);
        chk(ui, &mut v.portamento);
        dv(ui, &mut v.fc_volume,             0, 99);
        chk(ui, &mut v.sustain);
        dv(ui, &mut v.mw_pitch,             0, 99);
        dv(ui, &mut v.mw_amplitude,         0, 99);
        dv(ui, &mut v.bc_pitch,             0, 99);
        dv(ui, &mut v.bc_amplitude,         0, 99);
        dv(ui, &mut v.bc_pitch_bias,        0, 99);
        dv(ui, &mut v.bc_eg_bias,           0, 99);
        chk(ui, &mut v.chorus);
        cb(ui, "transpose",  TRANSPOSE_TBL,  &mut v.transpose);
        ui.end_row();
    });
}
