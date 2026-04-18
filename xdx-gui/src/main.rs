use eframe::egui::{self, Color32, Grid, RichText};
use std::path::PathBuf;
use xdx_core::dx100::Dx100Voice;
use xdx_core::sysex::{dx100_decode_1voice, dx100_decode_32voice, dx100_encode_1voice, dx100_encode_32voice};
use xdx_midi::{MidiEvent, MidiManager};

static IVORY_EBONY_SYX: &[u8] = include_bytes!("../../testdata/syx/IvoryEbony.syx");
static ALL_VOICES_SYX:  &[u8] = include_bytes!("../../testdata/syx/all_voices.syx");

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
const POLY_MONO_TBL:  &[&str] = &["POLY","MONO"];

// DX100 has 24 internal voice slots (25-32 are unused in the VMEM format)
const DX100_BANK_VOICES: usize = 24;

// ── widget helpers ────────────────────────────────────────────────────────────

fn dv(ui: &mut egui::Ui, val: &mut u8, min: u8, max: u8) {
    ui.add(egui::DragValue::new(val).range(min..=max));
}

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

// ── State enums ───────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum SynthType { Dx100, Dx7 }

#[derive(Clone, Copy, PartialEq)]
enum SysExState {
    Idle,
    Fetch1Pending  { sent_at: f64 },
    Fetch32Pending { sent_at: f64 },
}

const FETCH_TIMEOUT_SECS: f64 = 5.0;

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
    synth_type: SynthType,
    // MIDI
    midi_manager:   MidiManager,
    midi_in_sel:    Option<String>,
    midi_out_sel:   Option<String>,
    show_midi_test: bool,
    sysex_state:     SysExState,
    sysex_out_flash: f64,
    sysex_in_flash:  f64,
    // 1-voice edit buffer
    voice:      Dx100Voice,
    name_buf:   String,
    file_path:  Option<PathBuf>,
    // 32-voice bank
    bank:           Vec<Dx100Voice>,
    bank_sel:       usize,
    bank_file_path: Option<PathBuf>,
    status:     String,
}

impl App {
    fn new() -> Self {
        let voice = dx100_decode_1voice(IVORY_EBONY_SYX).expect("1-voice decode failed");
        let name_buf = voice.name_str();
        let bank = dx100_decode_32voice(ALL_VOICES_SYX).expect("32-voice decode failed");
        Self {
            synth_type: SynthType::Dx100,
            midi_manager:    MidiManager::new(),
            midi_in_sel:     None,
            midi_out_sel:    None,
            show_midi_test:  false,
            sysex_state:     SysExState::Idle,
            sysex_out_flash: f64::NEG_INFINITY,
            sysex_in_flash:  f64::NEG_INFINITY,
            voice,
            name_buf,
            file_path:       None,
            bank,
            bank_sel:        0,
            bank_file_path:  None,
            status: "Test data loaded".to_string(),
        }
    }

    // ── 1-voice file I/O ──────────────────────────────────────────────────────

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

    // ── 32-voice bank file I/O ────────────────────────────────────────────────

    fn open_bank_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SysEx", &["syx"])
            .pick_file()
        else { return };
        match std::fs::read(&path) {
            Err(e) => self.status = format!("Open failed: {e}"),
            Ok(bytes) => match dx100_decode_32voice(&bytes) {
                Err(e) => self.status = format!("Decode failed: {e:?}"),
                Ok(voices) => {
                    self.bank = voices;
                    self.bank_sel = 0;
                    self.status = format!("Opened bank: {}", path.display());
                    self.bank_file_path = Some(path);
                }
            },
        }
    }

    fn save_bank_file(&mut self) {
        let path = if let Some(p) = &self.bank_file_path {
            p.clone()
        } else {
            let Some(p) = rfd::FileDialog::new()
                .add_filter("SysEx", &["syx"])
                .set_file_name("bank.syx")
                .save_file()
            else { return };
            p
        };
        self.write_bank_file(path);
    }

    fn save_bank_as(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SysEx", &["syx"])
            .set_file_name("bank.syx")
            .save_file()
        else { return };
        self.write_bank_file(path);
    }

    fn write_bank_file(&mut self, path: PathBuf) {
        let bytes = dx100_encode_32voice(&self.bank, 0);
        match std::fs::write(&path, &bytes) {
            Err(e) => self.status = format!("Save failed: {e}"),
            Ok(()) => {
                self.status = format!("Saved bank: {}", path.display());
                self.bank_file_path = Some(path);
            }
        }
    }

    // ── MIDI helpers ──────────────────────────────────────────────────────────

    fn ensure_out(&mut self) -> Result<(), String> {
        if self.midi_manager.out_connected() { return Ok(()); }
        let name = self.midi_out_sel.clone()
            .ok_or_else(|| "No MIDI OUT device selected (Settings > MIDI OUT)".to_string())?;
        self.midi_manager.open_out(&name).map_err(|e| e.to_string())
    }

    fn ensure_in(&mut self) -> Result<(), String> {
        if self.midi_manager.in_connected() { return Ok(()); }
        let name = self.midi_in_sel.clone()
            .ok_or_else(|| "No MIDI IN device selected (Settings > MIDI IN)".to_string())?;
        self.midi_manager.open_in(&name).map_err(|e| e.to_string())
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.midi_manager.close_in();
        self.midi_manager.close_out();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = ctx.input(|i| i.time);
        const FLASH_SECS: f64 = 0.5;

        // ── Poll MIDI IN ──────────────────────────────────────────────────────
        while let Some(event) = self.midi_manager.try_recv() {
            if let MidiEvent::SysEx(bytes) = event {
                self.sysex_in_flash = now;
                if bytes.len() >= 4 && bytes[3] == 0x04 {
                    match dx100_decode_32voice(&bytes) {
                        Ok(voices) => {
                            self.bank = voices;
                            self.bank_sel = 0;
                            self.status = "Fetch 32: received 32-voice bank from synth".to_string();
                        }
                        Err(e) => {
                            self.status = format!("Fetch 32 decode error: {e:?}");
                        }
                    }
                } else {
                    match dx100_decode_1voice(&bytes) {
                        Ok(voice) => {
                            let name = voice.name_str();
                            self.name_buf = name.clone();
                            self.voice = voice;
                            self.status = format!("Fetch 1: received \"{name}\" from synth");
                        }
                        Err(e) => {
                            self.status = format!("Fetch 1 decode error: {e:?}");
                        }
                    }
                }
                self.midi_manager.close_in();
                self.midi_manager.close_out();
                self.sysex_state = SysExState::Idle;
            }
        }

        // ── Fetch timeout ─────────────────────────────────────────────────────
        let fetch_sent_at = match self.sysex_state {
            SysExState::Fetch1Pending  { sent_at } => Some(sent_at),
            SysExState::Fetch32Pending { sent_at } => Some(sent_at),
            SysExState::Idle => None,
        };
        if let Some(sent_at) = fetch_sent_at {
            if now - sent_at > FETCH_TIMEOUT_SECS {
                self.midi_manager.close_in();
                self.midi_manager.close_out();
                self.sysex_state = SysExState::Idle;
                self.status = format!("Fetch timeout: no response from device ({FETCH_TIMEOUT_SECS:.0}s)");
            }
        }

        if (now - self.sysex_in_flash) < FLASH_SECS
            || (now - self.sysex_out_flash) < FLASH_SECS
            || !matches!(self.sysex_state, SysExState::Idle)
        {
            ctx.request_repaint();
        }

        // ── Menu bar ─────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Settings", |ui| {
                    ui.menu_button("MIDI IN", |ui| {
                        let ports = MidiManager::list_in_ports();
                        if ports.is_empty() { ui.weak("(no devices found)"); }
                        for name in ports {
                            let sel = self.midi_in_sel.as_deref() == Some(name.as_str());
                            if ui.selectable_label(sel, &name).clicked() {
                                self.midi_in_sel = if sel { None } else { Some(name) };
                                ui.close_menu();
                            }
                        }
                    });
                    ui.menu_button("MIDI OUT", |ui| {
                        let ports = MidiManager::list_out_ports();
                        if ports.is_empty() { ui.weak("(no devices found)"); }
                        for name in ports {
                            let sel = self.midi_out_sel.as_deref() == Some(name.as_str());
                            if ui.selectable_label(sel, &name).clicked() {
                                self.midi_out_sel = if sel { None } else { Some(name) };
                                ui.close_menu();
                            }
                        }
                    });
                    ui.separator();
                    if ui.button("MIDI Device Test").clicked() {
                        self.show_midi_test = true;
                        ui.close_menu();
                    }
                });
            });
        });

        // ── Toolbar: SYNTH type + connection indicators ───────────────────────
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(hdr("SYNTH:"));
                ui.selectable_value(&mut self.synth_type, SynthType::Dx100, "DX100");
                ui.selectable_value(&mut self.synth_type, SynthType::Dx7,   "DX7");
                ui.separator();
                let in_flash  = (now - self.sysex_in_flash)  < FLASH_SECS;
                let out_flash = (now - self.sysex_out_flash) < FLASH_SECS;
                let dot = |connected: bool, flash: bool| -> RichText {
                    let color = if flash          { Color32::YELLOW }
                                else if connected { Color32::GREEN }
                                else              { Color32::from_gray(110) };
                    RichText::new("●").color(color)
                };
                let in_name  = self.midi_manager.in_port_name.as_deref()
                    .or(self.midi_in_sel.as_deref()).unwrap_or("(none)");
                let out_name = self.midi_manager.out_port_name.as_deref()
                    .or(self.midi_out_sel.as_deref()).unwrap_or("(none)");
                ui.label(dot(self.midi_manager.in_connected(),  in_flash));
                ui.label(format!("IN: {in_name}"));
                ui.label(dot(self.midi_manager.out_connected(), out_flash));
                ui.label(format!("OUT: {out_name}"));
            });
        });

        // ── Status bar ───────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.label(&self.status);
        });

        // ── MIDI Device Test window ───────────────────────────────────────────
        let mut show_midi_test = self.show_midi_test;
        if show_midi_test {
            egui::Window::new("MIDI Device Test")
                .resizable(false)
                .collapsible(false)
                .open(&mut show_midi_test)
                .show(ctx, |ui| {
                    Grid::new("midi_test_grid").num_columns(4).spacing([8.0, 6.0]).show(ui, |ui| {
                        ui.label(hdr("MIDI IN"));
                        let in_name = self.midi_in_sel.as_deref().unwrap_or("(not selected)");
                        ui.label(in_name);
                        if self.midi_manager.in_connected() {
                            if ui.button("Close").clicked() { self.midi_manager.close_in(); }
                            ui.label(RichText::new("OK").color(Color32::GREEN).strong());
                        } else {
                            let can = self.midi_in_sel.is_some();
                            if ui.add_enabled(can, egui::Button::new("Open")).clicked() {
                                if let Some(name) = self.midi_in_sel.clone() {
                                    if let Err(e) = self.midi_manager.open_in(&name) {
                                        self.status = format!("MIDI IN open failed: {e}");
                                    }
                                }
                            }
                            ui.label(RichText::new("--").weak());
                        }
                        ui.end_row();

                        ui.label(hdr("MIDI OUT"));
                        let out_name = self.midi_out_sel.as_deref().unwrap_or("(not selected)");
                        ui.label(out_name);
                        if self.midi_manager.out_connected() {
                            if ui.button("Close").clicked() { self.midi_manager.close_out(); }
                            ui.label(RichText::new("OK").color(Color32::GREEN).strong());
                        } else {
                            let can = self.midi_out_sel.is_some();
                            if ui.add_enabled(can, egui::Button::new("Open")).clicked() {
                                if let Some(name) = self.midi_out_sel.clone() {
                                    if let Err(e) = self.midi_manager.open_out(&name) {
                                        self.status = format!("MIDI OUT open failed: {e}");
                                    }
                                }
                            }
                            ui.label(RichText::new("--").weak());
                        }
                        ui.end_row();
                    });
                });
        }
        if self.show_midi_test && !show_midi_test {
            self.midi_manager.close_in();
            self.midi_manager.close_out();
        }
        self.show_midi_test = show_midi_test;

        // ── 32-voice bank panel (left) ────────────────────────────────────────
        egui::SidePanel::left("bank_panel")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.label(RichText::new("32 VOICES").strong().small());

                // FILE row
                ui.horizontal(|ui| {
                    ui.label(hdr("FILE:"));
                    if ui.button("Open").clicked()    { self.open_bank_file(); }
                    if ui.button("Save").clicked()    { self.save_bank_file(); }
                    if ui.button("Save As").clicked() { self.save_bank_as(); }
                });
                let bankname = self.bank_file_path.as_deref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("(test data)");
                ui.label(RichText::new(bankname).small().weak());

                // SysEx row
                ui.horizontal(|ui| {
                    ui.label(hdr("SysEx:"));
                    let is_fetch32 = matches!(self.sysex_state, SysExState::Fetch32Pending { .. });
                    let any_fetch  = !matches!(self.sysex_state, SysExState::Idle);

                    if is_fetch32 {
                        if ui.button("Cancel").clicked() {
                            self.midi_manager.close_in();
                            self.midi_manager.close_out();
                            self.sysex_state = SysExState::Idle;
                            self.status = "Fetch 32 cancelled".to_string();
                        }
                    } else {
                        if ui.add_enabled(!any_fetch, egui::Button::new("Fetch")).clicked() {
                            let result = self.ensure_out()
                                .and_then(|_| self.ensure_in())
                                .and_then(|_| self.midi_manager
                                    .send(&[0xF0, 0x43, 0x20, 0x04, 0xF7])
                                    .map_err(|e| e.to_string()));
                            match result {
                                Ok(()) => {
                                    self.sysex_out_flash = now;
                                    self.sysex_state = SysExState::Fetch32Pending { sent_at: now };
                                    self.status = "Fetch 32: request sent, waiting...".to_string();
                                }
                                Err(e) => self.status = format!("Fetch 32 failed: {e}"),
                            }
                        }
                    }

                    if ui.add_enabled(!any_fetch, egui::Button::new("Send")).clicked() {
                        let bytes = dx100_encode_32voice(&self.bank, 0);
                        let result = self.ensure_out()
                            .and_then(|_| self.midi_manager.send(&bytes)
                                .map_err(|e| e.to_string()));
                        match result {
                            Ok(()) => {
                                self.sysex_out_flash = now;
                                self.status = "Send 32: bank sent to synth".to_string();
                            }
                            Err(e) => self.status = format!("Send 32 failed: {e}"),
                        }
                        self.midi_manager.close_out();
                    }
                });

                ui.separator();

                // Voice list: DX100 uses slots 1-24 only
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let count = DX100_BANK_VOICES.min(self.bank.len());
                    for i in 0..count {
                        let name  = self.bank[i].name_str();
                        let label = format!("{:02}  {}", i + 1, name);
                        if ui.selectable_label(self.bank_sel == i, label).clicked() {
                            self.bank_sel = i;
                        }
                    }
                });
            });

        // ── Transfer panel (middle) ───────────────────────────────────────────
        egui::SidePanel::left("transfer_panel")
            .resizable(false)
            .default_width(44.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    if ui.button("->")
                        .on_hover_text("Copy selected bank voice to editor")
                        .clicked()
                    {
                        if let Some(v) = self.bank.get(self.bank_sel) {
                            self.voice   = v.clone();
                            self.name_buf = self.voice.name_str();
                            self.status  = format!(
                                "Loaded {:02}: {}", self.bank_sel + 1, self.voice.name_str()
                            );
                        }
                    }
                    ui.add_space(4.0);
                    if ui.button("<-")
                        .on_hover_text("Copy editor voice to selected bank slot")
                        .clicked()
                    {
                        if self.bank_sel < self.bank.len() {
                            self.bank[self.bank_sel] = self.voice.clone();
                            self.status = format!(
                                "Saved to bank slot {:02}", self.bank_sel + 1
                            );
                        }
                    }
                });
            });

        // ── 1-voice editor (central panel) ───────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(RichText::new("1 VOICE").strong().small());

            // FILE row
            ui.horizontal(|ui| {
                ui.label(hdr("FILE:"));
                if ui.button("Open").clicked()    { self.open_file(); }
                if ui.button("Save").clicked()    { self.save_file(); }
                if ui.button("Save As").clicked() { self.save_as(); }
            });
            let filename = self.file_path.as_deref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("(test data)");
            ui.label(RichText::new(filename).small().weak());

            // SysEx row
            ui.horizontal(|ui| {
                ui.label(hdr("SysEx:"));
                let is_fetch1 = matches!(self.sysex_state, SysExState::Fetch1Pending { .. });
                let any_fetch = !matches!(self.sysex_state, SysExState::Idle);

                if is_fetch1 {
                    if ui.button("Cancel").clicked() {
                        self.midi_manager.close_in();
                        self.midi_manager.close_out();
                        self.sysex_state = SysExState::Idle;
                        self.status = "Fetch 1 cancelled".to_string();
                    }
                } else {
                    if ui.add_enabled(!any_fetch, egui::Button::new("Fetch")).clicked() {
                        let result = self.ensure_out()
                            .and_then(|_| self.ensure_in())
                            .and_then(|_| self.midi_manager
                                .send(&[0xF0, 0x43, 0x20, 0x03, 0xF7])
                                .map_err(|e| e.to_string()));
                        match result {
                            Ok(()) => {
                                self.sysex_out_flash = now;
                                self.sysex_state = SysExState::Fetch1Pending { sent_at: now };
                                self.status = "Fetch 1: request sent, waiting...".to_string();
                            }
                            Err(e) => self.status = format!("Fetch 1 failed: {e}"),
                        }
                    }
                }

                if ui.add_enabled(!any_fetch, egui::Button::new("Send")).clicked() {
                    let bytes = dx100_encode_1voice(&self.voice, 0);
                    let result = self.ensure_out()
                        .and_then(|_| self.midi_manager.send(&bytes)
                            .map_err(|e| e.to_string()));
                    match result {
                        Ok(()) => {
                            self.sysex_out_flash = now;
                            self.status = format!("Send 1: \"{}\" sent to synth", self.voice.name_str());
                        }
                        Err(e) => self.status = format!("Send 1 failed: {e}"),
                    }
                    self.midi_manager.close_in();
                    self.midi_manager.close_out();
                }
            });

            ui.separator();

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
