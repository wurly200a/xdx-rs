#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;

use eframe::egui::{self, Color32, Grid, RichText};
use std::path::PathBuf;
use xdx_core::dx100::{Dx100Voice, BANK_VOICES, FREQ_RATIOS};
use xdx_core::sysex::{
    dx100_decode_1voice, dx100_decode_32voice, dx100_encode_1voice, dx100_encode_32voice,
};
use xdx_midi::{MidiEvent, MidiManager};
use xdx_synth::FmEngine;

// ── PC keyboard → MIDI note mapping (standard QWERTY piano layout) ────────────
// Lower row  Z..M  : C4(60)–B4(71)   upper row  Q..U  : C5(72)–B5(83)
// Black keys: S=C#4 D=D#4 G=F#4 H=G#4 J=A#4 / 2=C#5 3=D#5 5=F#5 6=G#5 7=A#5
const PIANO_KEYS: &[(egui::Key, u8)] = &[
    (egui::Key::Z, 60),
    (egui::Key::S, 61),
    (egui::Key::X, 62),
    (egui::Key::D, 63),
    (egui::Key::C, 64),
    (egui::Key::V, 65),
    (egui::Key::G, 66),
    (egui::Key::B, 67),
    (egui::Key::H, 68),
    (egui::Key::N, 69),
    (egui::Key::J, 70),
    (egui::Key::M, 71),
    (egui::Key::Q, 72),
    (egui::Key::Num2, 73),
    (egui::Key::W, 74),
    (egui::Key::Num3, 75),
    (egui::Key::E, 76),
    (egui::Key::R, 77),
    (egui::Key::Num5, 78),
    (egui::Key::T, 79),
    (egui::Key::Num6, 80),
    (egui::Key::Y, 81),
    (egui::Key::Num7, 82),
    (egui::Key::U, 83),
];

static IVORY_EBONY_SYX: &[u8] = include_bytes!("../../testdata/syx/IvoryEbony.syx");
static ALL_VOICES_SYX: &[u8] = include_bytes!("../../testdata/syx/all_voices.syx");

static ALGO_BMPS: [&[u8]; 8] = [
    include_bytes!("../assets/dx100_01.bmp"),
    include_bytes!("../assets/dx100_02.bmp"),
    include_bytes!("../assets/dx100_03.bmp"),
    include_bytes!("../assets/dx100_04.bmp"),
    include_bytes!("../assets/dx100_05.bmp"),
    include_bytes!("../assets/dx100_06.bmp"),
    include_bytes!("../assets/dx100_07.bmp"),
    include_bytes!("../assets/dx100_08.bmp"),
];

fn load_algo_textures(ctx: &egui::Context) -> Vec<egui::TextureHandle> {
    ALGO_BMPS
        .iter()
        .enumerate()
        .map(|(i, bytes)| {
            let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Bmp)
                .unwrap_or_else(|_| image::DynamicImage::new_rgba8(1, 1));
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let pixels: Vec<egui::Color32> = rgba
                .pixels()
                .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                .collect();
            ctx.load_texture(
                format!("algo_{i}"),
                egui::ColorImage { size, pixels },
                egui::TextureOptions::default(),
            )
        })
        .collect()
}

// ── lookup tables ─────────────────────────────────────────────────────────────
// FREQ_TBL is derived from xdx_core::dx100::FREQ_RATIOS at first use.
fn freq_tbl() -> &'static [String] {
    use std::sync::OnceLock;
    static TBL: OnceLock<Vec<String>> = OnceLock::new();
    TBL.get_or_init(|| FREQ_RATIOS.iter().map(|&r| format!("{r:.2}")).collect())
}

const DETUNE_TBL: &[&str] = &["-3", "-2", "-1", "0", "+1", "+2", "+3"];
const LFO_WAVE_TBL: &[&str] = &["SAW", "SQU", "TRI", "S/H"];
const ALGO_TBL: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8"];
const TRANSPOSE_TBL: &[&str] = &[
    "C 1", "C#1", "D 1", "D#1", "E 1", "F 1", "F#1", "G 1", "G#1", "A 1", "A#1", "B 1", "C 2",
    "C#2", "D 2", "D#2", "E 2", "F 2", "F#2", "G 2", "G#2", "A 2", "A#2", "B 2", "C 3", "C#3",
    "D 3", "D#3", "E 3", "F 3", "F#3", "G 3", "G#3", "A 3", "A#3", "B 3", "C 4", "C#4", "D 4",
    "D#4", "E 4", "F 4", "F#4", "G 4", "G#4", "A 4", "A#4", "B 4", "C 5",
];
const PORTA_MODE_TBL: &[&str] = &["Full", "Fing"];
const POLY_MONO_TBL: &[&str] = &["POLY", "MONO"];

// ── widget helpers ────────────────────────────────────────────────────────────

fn dv(ui: &mut egui::Ui, val: &mut u8, min: u8, max: u8) {
    ui.add(egui::DragValue::new(val).range(min..=max));
}

fn cb(ui: &mut egui::Ui, id: impl std::hash::Hash, tbl: &[impl AsRef<str>], val: &mut u8) {
    let selected = tbl.get(*val as usize).map(|s| s.as_ref()).unwrap_or("?");
    egui::ComboBox::from_id_source(id)
        .selected_text(selected)
        .width(52.0)
        .show_ui(ui, |ui| {
            for (i, label) in tbl.iter().enumerate() {
                ui.selectable_value(val, i as u8, label.as_ref());
            }
        });
}

fn chk(ui: &mut egui::Ui, val: &mut u8) {
    let mut b = *val != 0;
    if ui.checkbox(&mut b, "").changed() {
        *val = b as u8;
    }
}

fn hdr(s: &str) -> RichText {
    RichText::new(s).strong().small()
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .strong()
            .small()
            .color(egui::Color32::from_gray(160)),
    );
}

// ── Waveform preview data ─────────────────────────────────────────────────────

const WV_WIN_MS: f32 = 10.0; // RMS window size in ms (must match render_wv_bins)

#[derive(Default)]
struct WvBins {
    bins: Vec<f32>,
    onset: usize,
    peak: f32,
    hold_bins: usize,
}

struct EgAnchor {
    t_ms: f32,
    level: f32, // 0.0 = silence, 1.0 = attack peak
}

// ── State enums ───────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum SynthType {
    Dx100,
    Dx7,
}

#[derive(Clone, Copy, PartialEq)]
enum SysExState {
    Idle,
    Fetch1Pending { sent_at: f64 },
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
    eframe::run_native("xdx", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}

struct App {
    synth_type: SynthType,
    // Software synth
    audio: Option<audio::AudioHandle>,
    // MIDI keyboard routing
    midi_kbd_active: bool, // keep MIDI IN open; route NoteOn/Off → softsynth
    // MIDI
    midi_manager: MidiManager,
    midi_in_sel: Option<String>,
    midi_out_sel: Option<String>,
    show_midi_test: bool,
    sysex_state: SysExState,
    sysex_out_flash: f64,
    sysex_in_flash: f64,
    // MIDI port list cache (populated by background scan thread)
    in_ports: Vec<String>,
    out_ports: Vec<String>,
    scanning: bool,
    scan_rx: Option<std::sync::mpsc::Receiver<(Vec<String>, Vec<String>)>>,
    scan_started: Option<std::time::Instant>,
    // 1-voice edit buffer
    voice: Dx100Voice,
    name_buf: String,
    file_path: Option<PathBuf>,
    // 32-voice bank
    bank: Vec<Dx100Voice>,
    bank_sel: usize,
    bank_file_path: Option<PathBuf>,
    status: String,
    algo_textures: Vec<egui::TextureHandle>,
    pc_kbd_notes: std::collections::HashSet<u8>,
    voice_dirty: bool, // true → push voice to audio engine on next frame
    midi_ch: u8,       // 0-15 (displayed as 1-16)
    wv_note: u8,
    wv_hold_ms: u32,
    wv_dirty: bool,
    wv_ops: [WvBins; 4],
    wv_final: WvBins,
    wv_eg_ops: [Vec<EgAnchor>; 4],
    wv_zoom: f32,
    wv_pan: f32,
    wv_content_w: f32,
    wv_show_eg: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let voice = dx100_decode_1voice(IVORY_EBONY_SYX).expect("1-voice decode failed");
        let name_buf = voice.name_str();
        let bank = dx100_decode_32voice(ALL_VOICES_SYX).expect("32-voice decode failed");
        let audio = audio::AudioHandle::start().ok();
        let algo_textures = load_algo_textures(&cc.egui_ctx);
        let mut app = Self {
            synth_type: SynthType::Dx100,
            audio,
            midi_kbd_active: false,
            midi_manager: MidiManager::new(),
            midi_in_sel: None,
            midi_out_sel: None,
            show_midi_test: false,
            sysex_state: SysExState::Idle,
            sysex_out_flash: f64::NEG_INFINITY,
            sysex_in_flash: f64::NEG_INFINITY,
            in_ports: Vec::new(),
            out_ports: Vec::new(),
            scanning: false,
            scan_rx: None,
            scan_started: None,
            voice,
            name_buf,
            file_path: None,
            bank,
            bank_sel: 0,
            bank_file_path: None,
            status: "Test data loaded".to_string(),
            algo_textures,
            pc_kbd_notes: std::collections::HashSet::new(),
            voice_dirty: true, // push initial voice on first frame
            midi_ch: 0,
            wv_note: 60,
            wv_hold_ms: 3000,
            wv_dirty: true,
            wv_ops: Default::default(),
            wv_final: WvBins::default(),
            wv_eg_ops: Default::default(),
            wv_zoom: 1.0,
            wv_pan: 0.0,
            wv_content_w: 800.0,
            wv_show_eg: true,
        };
        app.start_port_scan(); // scan in background at startup
        app
    }

    /// Spawn a background thread to enumerate MIDI ports.
    /// The GUI thread never blocks waiting for WinMM.
    /// If a previous scan is still running (hung), it is abandoned and a new
    /// thread is started — the old thread will eventually be cleaned up on exit.
    fn start_port_scan(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let inp = MidiManager::list_in_ports();
            let outp = MidiManager::list_out_ports();
            let _ = tx.send((inp, outp));
        });
        self.scan_rx = Some(rx);
        self.scanning = true;
        self.scan_started = Some(std::time::Instant::now());
    }

    // ── 1-voice file I/O ──────────────────────────────────────────────────────

    fn open_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SysEx", &["syx"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read(&path) {
            Err(e) => self.status = format!("Open failed: {e}"),
            Ok(bytes) => match dx100_decode_1voice(&bytes) {
                Err(e) => self.status = format!("Decode failed: {e:?}"),
                Ok(voice) => {
                    self.name_buf = voice.name_str();
                    self.voice = voice;
                    self.voice_dirty = true;
                    self.wv_dirty = true;
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
            else {
                return;
            };
            p
        };
        self.write_file(path);
    }

    fn save_as(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SysEx", &["syx"])
            .set_file_name(format!("{}.syx", self.voice.name_str().trim()))
            .save_file()
        else {
            return;
        };
        self.write_file(path);
    }

    fn write_file(&mut self, path: PathBuf) {
        let bytes = dx100_encode_1voice(&self.voice, self.midi_ch);
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
        else {
            return;
        };
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
            else {
                return;
            };
            p
        };
        self.write_bank_file(path);
    }

    fn save_bank_as(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SysEx", &["syx"])
            .set_file_name("bank.syx")
            .save_file()
        else {
            return;
        };
        self.write_bank_file(path);
    }

    fn write_bank_file(&mut self, path: PathBuf) {
        let bytes = dx100_encode_32voice(&self.bank, self.midi_ch);
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
        if self.midi_manager.out_connected() {
            return Ok(());
        }
        let name = self
            .midi_out_sel
            .clone()
            .ok_or_else(|| "No MIDI OUT device selected (Settings > MIDI OUT)".to_string())?;
        self.midi_manager.open_out(&name).map_err(|e| e.to_string())
    }

    fn ensure_in(&mut self) -> Result<(), String> {
        if self.midi_manager.in_connected() {
            return Ok(());
        }
        let name = self
            .midi_in_sel
            .clone()
            .ok_or_else(|| "No MIDI IN device selected (Settings > MIDI IN)".to_string())?;
        self.midi_manager.open_in(&name).map_err(|e| e.to_string())
    }

    fn refresh_waveforms(&mut self) {
        let voice = self.voice.clone();
        let note = self.wv_note;
        let hold_ms = self.wv_hold_ms;
        self.wv_final = render_wv_bins(&voice, note, hold_ms);
        for op_idx in 0..4 {
            self.wv_ops[op_idx] = render_op_bins(&voice, op_idx, note, hold_ms);
            self.wv_eg_ops[op_idx] = compute_eg_anchors(&voice.ops[op_idx], note, hold_ms);
        }
        self.wv_dirty = false;
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
            match event {
                MidiEvent::SysEx(bytes) => {
                    self.sysex_in_flash = now;
                    if bytes.len() >= 4 && bytes[3] == 0x04 {
                        match dx100_decode_32voice(&bytes) {
                            Ok(voices) => {
                                self.bank = voices;
                                self.bank_sel = 0;
                                self.status =
                                    "Fetch 32: received 32-voice bank from synth".to_string();
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
                                self.voice_dirty = true;
                                self.status = format!("Fetch 1: received \"{name}\" from synth");
                            }
                            Err(e) => {
                                self.status = format!("Fetch 1 decode error: {e:?}");
                            }
                        }
                    }
                    // Keep IN open if keyboard mode is active; always close OUT
                    self.midi_manager.close_out();
                    if !self.midi_kbd_active {
                        self.midi_manager.close_in();
                    }
                    self.sysex_state = SysExState::Idle;
                }
                MidiEvent::Other(msg) => {
                    // Route Note On / Note Off from external MIDI keyboard → softsynth
                    if msg.len() >= 3 {
                        let note = msg[1];
                        let vel = msg[2];
                        match msg[0] & 0xF0 {
                            0x90 if vel > 0 => {
                                if let Some(ref a) = self.audio {
                                    a.note_on(note, vel);
                                }
                            }
                            0x80 | 0x90 => {
                                if let Some(ref a) = self.audio {
                                    a.note_off(note);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // ── Keep MIDI IN open for keyboard when active ────────────────────────
        if self.midi_kbd_active && !self.midi_manager.in_connected() {
            if let Some(name) = self.midi_in_sel.clone() {
                if let Err(e) = self.midi_manager.open_in(&name) {
                    self.status = format!("MIDI KBD IN failed: {e}");
                    self.midi_kbd_active = false;
                }
            } else {
                self.midi_kbd_active = false;
            }
        }

        // ── Fetch timeout ─────────────────────────────────────────────────────
        let fetch_sent_at = match self.sysex_state {
            SysExState::Fetch1Pending { sent_at } => Some(sent_at),
            SysExState::Fetch32Pending { sent_at } => Some(sent_at),
            SysExState::Idle => None,
        };
        if let Some(sent_at) = fetch_sent_at {
            if now - sent_at > FETCH_TIMEOUT_SECS {
                self.midi_manager.close_in();
                self.midi_manager.close_out();
                self.sysex_state = SysExState::Idle;
                self.status =
                    format!("Fetch timeout: no response from device ({FETCH_TIMEOUT_SECS:.0}s)");
            }
        }

        if (now - self.sysex_in_flash) < FLASH_SECS
            || (now - self.sysex_out_flash) < FLASH_SECS
            || !matches!(self.sysex_state, SysExState::Idle)
            || self.scanning
        {
            ctx.request_repaint();
        }

        // ── Software synth: push voice only when changed ─────────────────────────
        if self.voice_dirty {
            if let Some(ref audio) = self.audio {
                audio.set_voice(self.voice.clone());
            }
            self.voice_dirty = false;
        }

        // ── Waveform preview: re-render when dirty ───────────────────────────────
        if self.wv_dirty {
            self.refresh_waveforms();
        }

        // ── PC keyboard → Softsynth + MIDI OUT ───────────────────────────────────
        if !ctx.wants_keyboard_input() {
            let mut pressed = Vec::new();
            let mut released = Vec::new();
            ctx.input(|i| {
                for &(key, note) in PIANO_KEYS {
                    let down = i.key_down(key);
                    if down && !self.pc_kbd_notes.contains(&note) {
                        pressed.push(note);
                    }
                    if !down && self.pc_kbd_notes.contains(&note) {
                        released.push(note);
                    }
                }
            });
            for &n in &pressed {
                self.pc_kbd_notes.insert(n);
            }
            for &n in &released {
                self.pc_kbd_notes.remove(&n);
            }
            for note in pressed {
                if let Some(ref a) = self.audio {
                    a.note_on(note, 100);
                }
                if self.midi_manager.out_connected() {
                    let _ = self.midi_manager.send(&[0x90 | self.midi_ch, note, 100]);
                }
            }
            for note in released {
                if let Some(ref a) = self.audio {
                    a.note_off(note);
                }
                if self.midi_manager.out_connected() {
                    let _ = self.midi_manager.send(&[0x80 | self.midi_ch, note, 0]);
                }
            }
            if self.audio.is_some() || self.midi_manager.out_connected() {
                ctx.request_repaint();
            }
        }

        // Request repaint while MIDI keyboard is active (to keep polling IN)
        if self.midi_kbd_active {
            ctx.request_repaint();
        }

        // ── Collect background port-scan results (with timeout) ───────────────
        const SCAN_TIMEOUT_SECS: f64 = 5.0;
        if let Some(rx) = &self.scan_rx {
            if let Ok((inp, outp)) = rx.try_recv() {
                self.in_ports = inp;
                self.out_ports = outp;
                self.scan_rx = None;
                self.scanning = false;
                self.scan_started = None;
            } else if self
                .scan_started
                .map(|t| t.elapsed().as_secs_f64() > SCAN_TIMEOUT_SECS)
                .unwrap_or(false)
            {
                // WinMM hung — abandon the thread and allow retry
                self.scan_rx = None;
                self.scanning = false;
                self.scan_started = None;
                self.status =
                    "MIDI port scan timed out (try: net stop audiosrv & net start audiosrv)"
                        .to_string();
            }
        }

        // ── Menu bar ─────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Settings", |ui| {
                    // Port lists come from the cache — never blocks the GUI thread.
                    ui.menu_button("MIDI IN", |ui| {
                        if self.in_ports.is_empty() {
                            ui.weak(if self.scanning {
                                "(scanning...)"
                            } else {
                                "(no devices found)"
                            });
                        }
                        for name in self.in_ports.clone() {
                            let sel = self.midi_in_sel.as_deref() == Some(name.as_str());
                            if ui.selectable_label(sel, &name).clicked() {
                                self.midi_in_sel = if sel { None } else { Some(name) };
                                ui.close_menu();
                            }
                        }
                    });
                    ui.menu_button("MIDI OUT", |ui| {
                        if self.out_ports.is_empty() {
                            ui.weak(if self.scanning {
                                "(scanning...)"
                            } else {
                                "(no devices found)"
                            });
                        }
                        for name in self.out_ports.clone() {
                            let sel = self.midi_out_sel.as_deref() == Some(name.as_str());
                            if ui.selectable_label(sel, &name).clicked() {
                                self.midi_out_sel = if sel { None } else { Some(name) };
                                ui.close_menu();
                            }
                        }
                    });
                    ui.menu_button(format!("MIDI CH: {}", self.midi_ch + 1), |ui| {
                        for ch in 0u8..16 {
                            if ui
                                .selectable_label(self.midi_ch == ch, format!("{}", ch + 1))
                                .clicked()
                            {
                                self.midi_ch = ch;
                                ui.close_menu();
                            }
                        }
                    });
                    ui.separator();
                    let scan_label = if self.scanning {
                        "Scanning..."
                    } else {
                        "Scan Ports"
                    };
                    if ui.button(scan_label).clicked() {
                        self.start_port_scan(); // always allowed; abandons any hung scan
                        ui.close_menu();
                    }
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
                ui.selectable_value(&mut self.synth_type, SynthType::Dx7, "DX7");
                ui.separator();
                let in_flash = (now - self.sysex_in_flash) < FLASH_SECS;
                let out_flash = (now - self.sysex_out_flash) < FLASH_SECS;
                let dot = |connected: bool, flash: bool| -> RichText {
                    let color = if flash {
                        Color32::YELLOW
                    } else if connected {
                        Color32::GREEN
                    } else {
                        Color32::from_gray(110)
                    };
                    RichText::new("●").color(color)
                };
                let in_name = self
                    .midi_manager
                    .in_port_name
                    .as_deref()
                    .or(self.midi_in_sel.as_deref())
                    .unwrap_or("(none)");
                let out_name = self
                    .midi_manager
                    .out_port_name
                    .as_deref()
                    .or(self.midi_out_sel.as_deref())
                    .unwrap_or("(none)");
                ui.label(dot(self.midi_manager.in_connected(), in_flash));
                ui.label(format!("IN: {in_name}"));
                ui.label(dot(self.midi_manager.out_connected(), out_flash));
                ui.label(format!("OUT: {out_name}"));
                ui.separator();
                let audio_ok = self.audio.is_some();
                ui.label(RichText::new("●").color(if audio_ok {
                    Color32::GREEN
                } else {
                    Color32::RED
                }));
                if audio_ok {
                    ui.label(RichText::new("Audio  Z..M / Q..U").small());
                } else {
                    ui.label(
                        RichText::new("Audio: no device")
                            .small()
                            .color(Color32::RED),
                    );
                }
                ui.separator();
                let kbd_dot = RichText::new("●").color(if self.midi_kbd_active {
                    Color32::GREEN
                } else {
                    Color32::from_gray(110)
                });
                ui.label(kbd_dot);
                let can_toggle = self.midi_in_sel.is_some();
                let btn_text = if self.midi_kbd_active {
                    "MIDI KBD: ON"
                } else {
                    "MIDI KBD: OFF"
                };
                if ui
                    .add_enabled(
                        can_toggle,
                        egui::Button::new(RichText::new(btn_text).small()),
                    )
                    .on_hover_text(
                        "Toggle MIDI IN open for keyboard input (requires MIDI IN device)",
                    )
                    .clicked()
                {
                    self.midi_kbd_active = !self.midi_kbd_active;
                    if !self.midi_kbd_active && matches!(self.sysex_state, SysExState::Idle) {
                        self.midi_manager.close_in();
                    }
                }
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
                    Grid::new("midi_test_grid")
                        .num_columns(4)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(hdr("MIDI IN"));
                            let in_name = self.midi_in_sel.as_deref().unwrap_or("(not selected)");
                            ui.label(in_name);
                            if self.midi_manager.in_connected() {
                                if ui.button("Close").clicked() {
                                    self.midi_manager.close_in();
                                }
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
                                if ui.button("Close").clicked() {
                                    self.midi_manager.close_out();
                                }
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
                    if ui.button("Open").clicked() {
                        self.open_bank_file();
                    }
                    if ui.button("Save").clicked() {
                        self.save_bank_file();
                    }
                    if ui.button("Save As").clicked() {
                        self.save_bank_as();
                    }
                });
                let bankname = self
                    .bank_file_path
                    .as_deref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("(test data)");
                ui.label(RichText::new(bankname).small().weak());

                // SysEx row
                ui.horizontal(|ui| {
                    ui.label(hdr("SysEx:"));
                    let is_fetch32 = matches!(self.sysex_state, SysExState::Fetch32Pending { .. });
                    let any_fetch = !matches!(self.sysex_state, SysExState::Idle);

                    if is_fetch32 {
                        if ui.button("Cancel").clicked() {
                            self.midi_manager.close_in();
                            self.midi_manager.close_out();
                            self.sysex_state = SysExState::Idle;
                            self.status = "Fetch 32 cancelled".to_string();
                        }
                    } else {
                        if ui
                            .add_enabled(!any_fetch, egui::Button::new("Fetch"))
                            .clicked()
                        {
                            let result =
                                self.ensure_out()
                                    .and_then(|_| self.ensure_in())
                                    .and_then(|_| {
                                        self.midi_manager
                                            .send(&[0xF0, 0x43, 0x20 | self.midi_ch, 0x04, 0xF7])
                                            .map_err(|e| e.to_string())
                                    });
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

                    if ui
                        .add_enabled(!any_fetch, egui::Button::new("Send"))
                        .clicked()
                    {
                        let bytes = dx100_encode_32voice(&self.bank, self.midi_ch);
                        let result = self.ensure_out().and_then(|_| {
                            self.midi_manager
                                .send_then_close(&bytes)
                                .map_err(|e| e.to_string())
                        });
                        match result {
                            Ok(()) => {
                                self.sysex_out_flash = now;
                                self.status = "Send 32: bank sent to synth".to_string();
                            }
                            Err(e) => self.status = format!("Send 32 failed: {e}"),
                        }
                    }
                });

                ui.separator();

                // Voice list: DX100 uses slots 1-24 only
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let count = BANK_VOICES.min(self.bank.len());
                    for i in 0..count {
                        let name = self.bank[i].name_str();
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
                    if ui
                        .button("->")
                        .on_hover_text("Copy selected bank voice to editor")
                        .clicked()
                    {
                        if let Some(v) = self.bank.get(self.bank_sel) {
                            self.voice = v.clone();
                            self.voice_dirty = true;
                            self.wv_dirty = true;
                            self.name_buf = self.voice.name_str();
                            self.status = format!(
                                "Loaded {:02}: {}",
                                self.bank_sel + 1,
                                self.voice.name_str()
                            );
                        }
                    }
                    ui.add_space(4.0);
                    if ui
                        .button("<-")
                        .on_hover_text("Copy editor voice to selected bank slot")
                        .clicked()
                    {
                        if self.bank_sel < self.bank.len() {
                            self.bank[self.bank_sel] = self.voice.clone();
                            self.status = format!("Saved to bank slot {:02}", self.bank_sel + 1);
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
                if ui.button("Open").clicked() {
                    self.open_file();
                }
                if ui.button("Save").clicked() {
                    self.save_file();
                }
                if ui.button("Save As").clicked() {
                    self.save_as();
                }
            });
            let filename = self
                .file_path
                .as_deref()
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
                    if ui
                        .add_enabled(!any_fetch, egui::Button::new("Fetch"))
                        .clicked()
                    {
                        let result =
                            self.ensure_out()
                                .and_then(|_| self.ensure_in())
                                .and_then(|_| {
                                    self.midi_manager
                                        .send(&[0xF0, 0x43, 0x20 | self.midi_ch, 0x03, 0xF7])
                                        .map_err(|e| e.to_string())
                                });
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

                if ui
                    .add_enabled(!any_fetch, egui::Button::new("Send"))
                    .clicked()
                {
                    let bytes = dx100_encode_1voice(&self.voice, self.midi_ch);
                    let result = self.ensure_out().and_then(|_| {
                        self.midi_manager
                            .send_then_close(&bytes)
                            .map_err(|e| e.to_string())
                    });
                    match result {
                        Ok(()) => {
                            self.sysex_out_flash = now;
                            self.status =
                                format!("Send 1: \"{}\" sent to synth", self.voice.name_str());
                        }
                        Err(e) => self.status = format!("Send 1 failed: {e}"),
                    }
                }
            });

            ui.separator();

            egui::ScrollArea::both().show(ui, |ui| {
                let before = self.voice.clone();
                let scope_resp = ui.scope(|ui| {
                    show_dx100_voice(ui, &mut self.voice, &mut self.name_buf, &self.algo_textures);
                });
                let content_w = scope_resp.response.rect.width();
                if content_w > 100.0 {
                    self.wv_content_w = content_w;
                }
                if self.voice != before {
                    self.voice_dirty = true;
                    self.wv_dirty = true;
                }

                ui.separator();

                // Waveform preview controls
                ui.horizontal(|ui| {
                    ui.label(hdr("PREVIEW:"));
                    ui.label("Note:");
                    let note_resp =
                        ui.add(egui::DragValue::new(&mut self.wv_note).range(0u8..=127u8));
                    ui.label(midi_note_name(self.wv_note));
                    if note_resp.changed() {
                        self.wv_dirty = true;
                    }
                    ui.separator();
                    ui.label("Hold:");
                    let hold_resp =
                        ui.add(egui::DragValue::new(&mut self.wv_hold_ms).range(100u32..=8000u32));
                    ui.label("ms");
                    if hold_resp.changed() {
                        self.wv_dirty = true;
                    }
                    ui.separator();
                    ui.label("Zoom:");
                    ui.add(
                        egui::DragValue::new(&mut self.wv_zoom)
                            .range(1.0f32..=16.0f32)
                            .speed(0.05),
                    );
                    ui.label("x");
                    if ui.small_button("Reset").clicked() {
                        self.wv_zoom = 1.0;
                        self.wv_pan = 0.0;
                    }
                    ui.separator();
                    ui.checkbox(&mut self.wv_show_eg, "EG");
                });

                ui.add_space(4.0);

                // Waveform panels — fixed width aligned to PL3 column (~260px for algo diagram)
                const OP_COLORS: [Color32; 4] = [
                    Color32::from_rgb(80, 180, 80),
                    Color32::from_rgb(80, 160, 220),
                    Color32::from_rgb(220, 180, 60),
                    Color32::from_rgb(200, 100, 80),
                ];
                const LABEL_W: f32 = 48.0;
                let wave_w = (self.wv_content_w - 260.0 - LABEL_W - 8.0).max(80.0);
                let zoom = self.wv_zoom;
                let show_eg = self.wv_show_eg;
                show_waveform(
                    ui,
                    &self.wv_final,
                    &[],
                    "FINAL",
                    Color32::WHITE,
                    wave_w,
                    zoom,
                    &mut self.wv_pan,
                    false,
                );
                for op_idx in (0..4usize).rev() {
                    let label = format!("OP{}", op_idx + 1);
                    // wv_pan is shared — borrow split needed
                    let pan = &mut self.wv_pan;
                    show_waveform(
                        ui,
                        &self.wv_ops[op_idx],
                        &self.wv_eg_ops[op_idx],
                        &label,
                        OP_COLORS[op_idx],
                        wave_w,
                        zoom,
                        pan,
                        show_eg,
                    );
                }
            });
        });
    }
}

// ── main panel ────────────────────────────────────────────────────────────────

fn show_dx100_voice(
    ui: &mut egui::Ui,
    v: &mut Dx100Voice,
    name_buf: &mut String,
    algo_textures: &[egui::TextureHandle],
) {
    let sp = [8.0_f32, 4.0_f32];

    ui.horizontal(|ui| {
        // ── Left: all params ─────────────────────────────────────────────────
        ui.vertical(|ui| {
            // ── PATCHNAME ─────────────────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.label(hdr("PATCHNAME"));
                let resp = ui.add(
                    egui::TextEdit::singleline(name_buf)
                        .desired_width(88.0)
                        .font(egui::TextStyle::Monospace),
                );
                if resp.changed() {
                    name_buf.truncate(10);
                    for (i, b) in v.name.iter_mut().enumerate() {
                        *b = name_buf.as_bytes().get(i).copied().unwrap_or(b' ');
                    }
                }
            });
            ui.add_space(4.0);

            // ── Global + per-operator AME / EG BIAS / VELOCITY ────────────────
            ui.horizontal(|ui| {
                ui.add_space(120.0);
                section_label(ui, "-------------- LFO --------------");
                ui.add_space(30.0);
                section_label(ui, "-- MODULATION SENSITIVITY --");
                ui.add_space(18.0);
                section_label(ui, "-- KEY --");
            });

            Grid::new("global")
                .num_columns(14)
                .spacing(sp)
                .show(ui, |ui| {
                    for h in &[
                        "ALGORITHM",
                        "FEEDBACK",
                        "WAVE",
                        "SPEED",
                        "DELAY",
                        "PMD",
                        "AMD",
                        "SYNC",
                        "PITCH",
                        "AMPLITUDE",
                        "AME",
                        "EG BIAS",
                        "VELOCITY",
                        "",
                    ] {
                        ui.label(hdr(h));
                    }
                    ui.end_row();

                    cb(ui, "algo", ALGO_TBL, &mut v.algorithm);
                    dv(ui, &mut v.feedback, 0, 7);
                    cb(ui, "lfowave", LFO_WAVE_TBL, &mut v.lfo_wave);
                    dv(ui, &mut v.lfo_speed, 0, 99);
                    dv(ui, &mut v.lfo_delay, 0, 99);
                    dv(ui, &mut v.lfo_pmd, 0, 99);
                    dv(ui, &mut v.lfo_amd, 0, 99);
                    chk(ui, &mut v.lfo_sync);
                    dv(ui, &mut v.pitch_mod_sens, 0, 7);
                    dv(ui, &mut v.amp_mod_sens, 0, 3);
                    chk(ui, &mut v.ops[3].amp_mod_en);
                    dv(ui, &mut v.ops[3].eg_bias_sens, 0, 7);
                    dv(ui, &mut v.ops[3].key_vel_sens, 0, 7);
                    ui.label(hdr("OPERATOR4"));
                    ui.end_row();

                    for (op_idx, label) in
                        [(2usize, "OPERATOR3"), (1, "OPERATOR2"), (0, "OPERATOR1")]
                    {
                        for _ in 0..10 {
                            ui.label("");
                        }
                        chk(ui, &mut v.ops[op_idx].amp_mod_en);
                        dv(ui, &mut v.ops[op_idx].eg_bias_sens, 0, 7);
                        dv(ui, &mut v.ops[op_idx].key_vel_sens, 0, 7);
                        ui.label(hdr(label));
                        ui.end_row();
                    }
                });

            ui.add_space(6.0);

            // ── OSCILLATOR + EG + KEY SCALING + PITCH EG ──────────────────────
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

            Grid::new("operators")
                .num_columns(17)
                .spacing(sp)
                .show(ui, |ui| {
                    for h in &[
                        "",
                        "RATIO",
                        "DETUNE",
                        "AR",
                        "D1R",
                        "D1L",
                        "D2R",
                        "RR",
                        "OUT LEVEL",
                        "RATE",
                        "LEVEL",
                        "PR1",
                        "PL1",
                        "PR2",
                        "PL2",
                        "PR3",
                        "PL3",
                    ] {
                        ui.label(hdr(h));
                    }
                    ui.end_row();

                    for (op_idx, label) in [
                        (3usize, "OPERATOR4"),
                        (2, "OPERATOR3"),
                        (1, "OPERATOR2"),
                        (0, "OPERATOR1"),
                    ] {
                        let op = &mut v.ops[op_idx];
                        ui.label(hdr(label));
                        cb(ui, ("freq", op_idx), freq_tbl(), &mut op.freq_ratio);
                        cb(ui, ("det", op_idx), DETUNE_TBL, &mut op.detune);
                        dv(ui, &mut op.ar, 0, 31);
                        dv(ui, &mut op.d1r, 0, 31);
                        dv(ui, &mut op.d1l, 0, 15);
                        dv(ui, &mut op.d2r, 0, 31);
                        dv(ui, &mut op.rr, 0, 15);
                        dv(ui, &mut op.out_level, 0, 99);
                        dv(ui, &mut op.kbd_rate_scl, 0, 3);
                        dv(ui, &mut op.kbd_lev_scl, 0, 99);
                        if op_idx == 0 {
                            dv(ui, &mut v.pitch_eg_rate[0], 0, 99);
                            dv(ui, &mut v.pitch_eg_level[0], 0, 99);
                            dv(ui, &mut v.pitch_eg_rate[1], 0, 99);
                            dv(ui, &mut v.pitch_eg_level[1], 0, 99);
                            dv(ui, &mut v.pitch_eg_rate[2], 0, 99);
                            dv(ui, &mut v.pitch_eg_level[2], 0, 99);
                        }
                        ui.end_row();
                    }
                });

            ui.add_space(6.0);

            // ── Performance controls ───────────────────────────────────────────
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

            Grid::new("perf")
                .num_columns(15)
                .spacing(sp)
                .show(ui, |ui| {
                    for h in &[
                        "POLY/MONO",
                        "RANGE",
                        "MODE",
                        "TIME",
                        "FOOT SW",
                        "VOLUME",
                        "SUSTAIN",
                        "PITCH",
                        "AMPLITUDE",
                        "PITCH",
                        "AMPLITUDE",
                        "PITCH BIAS",
                        "EG BIAS",
                        "CHORUS",
                        "TRANSPOSE",
                    ] {
                        ui.label(hdr(h));
                    }
                    ui.end_row();

                    cb(ui, "polymono", POLY_MONO_TBL, &mut v.poly_mono);
                    dv(ui, &mut v.pb_range, 0, 12);
                    cb(ui, "portamode", PORTA_MODE_TBL, &mut v.porta_mode);
                    dv(ui, &mut v.porta_time, 0, 99);
                    chk(ui, &mut v.portamento);
                    dv(ui, &mut v.fc_volume, 0, 99);
                    chk(ui, &mut v.sustain);
                    dv(ui, &mut v.mw_pitch, 0, 99);
                    dv(ui, &mut v.mw_amplitude, 0, 99);
                    dv(ui, &mut v.bc_pitch, 0, 99);
                    dv(ui, &mut v.bc_amplitude, 0, 99);
                    dv(ui, &mut v.bc_pitch_bias, 0, 99);
                    dv(ui, &mut v.bc_eg_bias, 0, 99);
                    chk(ui, &mut v.chorus);
                    cb(ui, "transpose", TRANSPOSE_TBL, &mut v.transpose);
                    ui.end_row();
                });
        }); // end left ui.vertical

        // ── Right: algorithm diagram (top-aligned with PATCHNAME) ────────────
        if let Some(tex) = algo_textures.get(v.algorithm as usize) {
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(hdr(&format!("ALGORITHM {}", v.algorithm + 1)));
                ui.add(
                    egui::Image::new(tex)
                        .max_size(egui::vec2(246.0, 151.0))
                        .maintain_aspect_ratio(true),
                );
            });
        }
    }); // end outer ui.horizontal
}

// ── Waveform preview helpers ──────────────────────────────────────────────────

fn midi_note_name(note: u8) -> String {
    const NAMES: &[&str] = &[
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let oct = (note as i32 / 12) - 1;
    format!("{}{}", NAMES[(note % 12) as usize], oct)
}

fn render_wv_bins(voice: &Dx100Voice, note: u8, hold_ms: u32) -> WvBins {
    const SR: f32 = 44100.0;
    const WIN_MS: f32 = 10.0;
    const RELEASE_MS: u32 = 1500;
    let win = (SR * WIN_MS / 1000.0) as usize;
    let hold_n = (SR * hold_ms as f32 / 1000.0) as usize;
    let rel_n = (SR * RELEASE_MS as f32 / 1000.0) as usize;

    let mut engine = FmEngine::new(SR);
    engine.set_voice(voice.clone());
    engine.note_on(note, 100);
    let mut buf = vec![0.0f32; hold_n + rel_n];
    if hold_n > 0 {
        engine.render(&mut buf[..hold_n]);
    }
    engine.note_off(note);
    if rel_n > 0 {
        engine.render(&mut buf[hold_n..]);
    }

    let bins: Vec<f32> = buf
        .chunks(win)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect();
    let hold_bins = hold_n / win;
    let peak = bins.iter().cloned().fold(0.0_f32, f32::max);
    let onset = {
        let thr = peak * 0.005;
        bins.iter().position(|&r| r > thr).unwrap_or(0)
    };
    WvBins {
        bins,
        onset,
        peak,
        hold_bins,
    }
}

fn render_op_bins(voice: &Dx100Voice, op_idx: usize, note: u8, hold_ms: u32) -> WvBins {
    // Render each OP as a standalone carrier (algorithm 1, no FM, no feedback)
    // so modulator EG shapes are also visible.
    let mut v = voice.clone();
    v.algorithm = 0;
    v.feedback = 0;
    v.ops[0] = voice.ops[op_idx].clone();
    for i in 1..4usize {
        v.ops[i].out_level = 0;
    }
    render_wv_bins(&v, note, hold_ms)
}

fn compute_eg_anchors(
    op: &xdx_core::dx100::Dx100Operator,
    note: u8,
    hold_ms: u32,
) -> Vec<EgAnchor> {
    let krs = op.kbd_rate_scl;
    let effective_krs = (krs * (krs + 1)) / 2;
    let rate_boost = ((effective_krs as f32 * note as f32 / 72.0).round()) as u8;
    let ar = (op.ar + rate_boost).min(31);
    let d1r = (op.d1r + rate_boost).min(31);
    let d2r = (op.d2r + rate_boost).min(31);
    let rr = (op.rr + rate_boost).min(15);

    let d1l: f32 = if op.d1l == 0 {
        0.0
    } else if op.d1l >= 15 {
        1.0
    } else {
        2.0f32.powf((op.d1l as f32 - 15.0) * 0.5)
    };

    // Half-life in ms for a decay rate; returns f32::INFINITY when rate==0.
    let hl_ms = |rate: u8, max: u8, coeff: f32| -> f32 {
        if rate == 0 {
            f32::INFINITY
        } else {
            coeff * 2.0f32.powf((max - rate) as f32 * 0.55) * 1000.0
        }
    };

    let mut pts = vec![EgAnchor {
        t_ms: 0.0,
        level: 0.0,
    }];
    let hold_f = hold_ms as f32;

    if ar == 0 {
        return pts; // no attack — stays at 0
    }
    let t_atk = 0.000085 * 2.0f32.powf((31 - ar) as f32 * 0.55) * 1000.0;
    if t_atk >= hold_f {
        pts.push(EgAnchor {
            t_ms: hold_f,
            level: (hold_f / t_atk).min(1.0),
        });
        return pts;
    }
    pts.push(EgAnchor {
        t_ms: t_atk,
        level: 1.0,
    });

    // D1 decay: 1.0 → d1l
    let t_hl_d1 = hl_ms(d1r, 31, 0.000092);
    let d1_dur = if t_hl_d1.is_infinite() || d1l >= 1.0 {
        f32::INFINITY
    } else {
        (1.0f32 / d1l.max(1e-7)).log2() * t_hl_d1
    };
    let t_d1_end = t_atk + d1_dur;

    let level_at_noff: f32;
    if t_d1_end <= hold_f {
        // D1 finished before note-off
        pts.push(EgAnchor {
            t_ms: t_d1_end,
            level: d1l,
        });
        // D2 decay from d1l until note-off
        let t_hl_d2 = hl_ms(d2r, 31, 0.000092);
        let d2_elapsed = hold_f - t_d1_end;
        level_at_noff = if t_hl_d2.is_infinite() {
            d1l
        } else {
            d1l * 0.5f32.powf(d2_elapsed / t_hl_d2)
        };
    } else {
        // D1 still decaying at note-off
        let d1_elapsed = hold_f - t_atk;
        level_at_noff = if t_hl_d1.is_infinite() {
            1.0
        } else {
            0.5f32.powf(d1_elapsed / t_hl_d1)
        };
    }

    // Note-off
    pts.push(EgAnchor {
        t_ms: hold_f,
        level: level_at_noff,
    });

    // Release tail — show for the full 1500 ms release window
    const RELEASE_WIN_MS: f32 = 1500.0;
    let t_hl_rr = hl_ms(rr, 15, 0.0014);
    let level_end = if t_hl_rr.is_infinite() {
        level_at_noff
    } else {
        level_at_noff * 0.5f32.powf(RELEASE_WIN_MS / t_hl_rr)
    };
    pts.push(EgAnchor {
        t_ms: hold_f + RELEASE_WIN_MS,
        level: level_end,
    });

    pts
}

fn show_waveform(
    ui: &mut egui::Ui,
    data: &WvBins,
    eg: &[EgAnchor],
    label: &str,
    color: Color32,
    wave_w: f32,
    zoom: f32,
    pan_bins: &mut f32,
    show_eg: bool,
) {
    const BG: Color32 = Color32::from_rgb(18, 18, 28);
    const BORDER: Color32 = Color32::from_rgb(50, 55, 70);
    const NOFF: Color32 = Color32::from_rgba_premultiplied(112, 112, 41, 130);
    const WV_H: f32 = 68.0;
    const LABEL_W: f32 = 48.0;

    let total = data.bins.len().saturating_sub(data.onset);
    let visible = ((total as f32 / zoom.max(1.0)).ceil() as usize).max(2);
    let max_pan = (total.saturating_sub(visible)) as f32;
    *pan_bins = pan_bins.clamp(0.0, max_pan);
    let start = pan_bins.floor() as usize;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width(LABEL_W);
            ui.set_min_height(WV_H);
            ui.add_space(WV_H / 2.0 - 8.0);
            ui.label(RichText::new(label).small().strong().color(color));
        });

        let (response, painter) =
            ui.allocate_painter(egui::vec2(wave_w, WV_H), egui::Sense::drag());
        let rect = response.rect;
        painter.rect_filled(rect, 2.0, BG);
        painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, BORDER));

        if response.dragged() {
            let delta = -response.drag_delta().x / wave_w * visible as f32;
            *pan_bins = (*pan_bins + delta).clamp(0.0, max_pan);
        }

        if total < 2 || data.peak < 1e-7 {
            return;
        }

        let mt = rect.height() * 0.05;
        let uh = rect.height() * 0.90;
        let to_x = |n: f32| rect.left() + (n / visible as f32).clamp(0.0, 1.0) * rect.width();
        let to_y = |v: f32| rect.top() + mt + (1.0 - (v / data.peak).clamp(0.0, 1.0)) * uh;

        // Note-off line
        let noff_disp = data
            .hold_bins
            .saturating_sub(data.onset)
            .saturating_sub(start);
        if noff_disp < visible {
            painter.line_segment(
                [
                    egui::pos2(to_x(noff_disp as f32), rect.top()),
                    egui::pos2(to_x(noff_disp as f32), rect.bottom()),
                ],
                egui::Stroke::new(1.0, NOFF),
            );
        }

        // RMS amplitude envelope
        let end = (start + visible).min(total);
        let pts: Vec<egui::Pos2> = (start..end)
            .enumerate()
            .map(|(i, b)| egui::pos2(to_x(i as f32), to_y(data.bins[data.onset + b])))
            .collect();
        if pts.len() >= 2 {
            painter.add(egui::Shape::line(pts, egui::Stroke::new(1.5, color)));
        }

        // Theoretical EG overlay
        if show_eg && eg.len() >= 2 {
            let eg_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 170);
            let anchor_to_pos = |a: &EgAnchor| -> Option<egui::Pos2> {
                let raw_bin = a.t_ms / WV_WIN_MS;
                let disp = raw_bin - data.onset as f32 - start as f32;
                if disp < -0.5 || disp > visible as f32 + 0.5 {
                    return None;
                }
                let x = rect.left() + (disp / visible as f32).clamp(0.0, 1.0) * rect.width();
                let y = rect.top() + mt + (1.0 - a.level.clamp(0.0, 1.0)) * uh;
                Some(egui::pos2(x, y))
            };
            let eg_pts: Vec<egui::Pos2> = eg.iter().filter_map(anchor_to_pos).collect();
            if eg_pts.len() >= 2 {
                painter.add(egui::Shape::line(eg_pts, egui::Stroke::new(1.0, eg_color)));
            }
            for a in eg.iter() {
                if let Some(p) = anchor_to_pos(a) {
                    painter.circle_filled(p, 2.5, eg_color);
                }
            }
        }
    });
}
