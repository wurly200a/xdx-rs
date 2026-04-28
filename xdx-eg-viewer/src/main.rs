#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke, Vec2};
use hound::WavReader;

const WINDOW_MS: f32 = 10.0;

// ── Colors ────────────────────────────────────────────────────────────────────

const HW_COLOR: Color32 = Color32::from_rgb(80, 150, 230);
const SY_COLOR: Color32 = Color32::from_rgb(230, 140, 40);
const NOFF_COLOR: Color32 = Color32::from_rgba_premultiplied(112, 112, 41, 130);
const BG_COLOR: Color32 = Color32::from_rgb(18, 18, 28);
const BORDER_COLOR: Color32 = Color32::from_rgb(50, 55, 70);

// ── Data loading (mirrors compare_eg logic) ───────────────────────────────────

fn load_rms_bins(path: &str) -> Option<Vec<f32>> {
    let mut reader = WavReader::open(path).ok()?;
    let sr = reader.spec().sample_rate as f32;
    let win = (sr * WINDOW_MS / 1000.0) as usize;
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();
    let bins = samples
        .chunks(win)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect();
    Some(bins)
}

fn find_onset(bins: &[f32]) -> usize {
    let peak = bins.iter().cloned().fold(0.0_f32, f32::max);
    let thr = peak * 0.005;
    bins.iter().position(|&r| r > thr).unwrap_or(0)
}

#[derive(Clone, Default)]
struct EgMetrics {
    atk90_ms: f32,
    d1l: f32,
    rls50_ms: f32,
    rls90_ms: f32,
}

fn compute_metrics(bins: &[f32], onset: usize, hold_bins: usize) -> EgMetrics {
    let peak = bins.iter().cloned().fold(0.0_f32, f32::max);
    if peak < 1e-7 {
        return EgMetrics {
            atk90_ms: f32::NAN,
            d1l: 0.0,
            rls50_ms: f32::NAN,
            rls90_ms: f32::NAN,
        };
    }
    let get = |n: usize| bins.get(onset + n).copied().unwrap_or(0.0) / peak;

    let atk90_ms = (0..hold_bins)
        .find(|&n| get(n) >= 0.9)
        .map(|n| n as f32 * WINDOW_MS)
        .unwrap_or(f32::NAN);

    let d1l_start = hold_bins * 9 / 10;
    let d1l_count = hold_bins.saturating_sub(d1l_start).max(1);
    let d1l = (0..d1l_count).map(|i| get(d1l_start + i)).sum::<f32>() / d1l_count as f32;

    let at_off = get(hold_bins);
    let rls_ms = |frac: f32| -> f32 {
        let thr = at_off * frac;
        (0..)
            .find(|&n| get(hold_bins + n) <= thr)
            .map(|n| n as f32 * WINDOW_MS)
            .unwrap_or(f32::NAN)
    };

    EgMetrics {
        atk90_ms,
        d1l,
        rls50_ms: rls_ms(0.5),
        rls90_ms: rls_ms(0.1),
    }
}

// ── Voice data ────────────────────────────────────────────────────────────────

struct VoiceRow {
    name: String,
    dx_bins: Vec<f32>,
    sy_bins: Vec<f32>,
    dx_onset: usize,
    sy_onset: usize,
    dx_peak: f32,
    sy_peak: f32,
    hold_bins: usize,
    dx_m: EgMetrics,
    sy_m: EgMetrics,
}

// ── App ───────────────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum ViewMode {
    Overlay,
    SideBySide,
}

struct EgViewerApp {
    dir: String,
    hold_ms_str: String,
    hold_ms: f32,
    rows: Vec<(usize, Option<VoiceRow>)>,
    status: String,
    view_mode: ViewMode,
    wave_height: f32,
}

impl Default for EgViewerApp {
    fn default() -> Self {
        Self {
            dir: "out/eg_bank".to_string(),
            hold_ms_str: "3000".to_string(),
            hold_ms: 3000.0,
            rows: Vec::new(),
            status: String::new(),
            view_mode: ViewMode::Overlay,
            wave_height: 88.0,
        }
    }
}

impl EgViewerApp {
    fn new_with_args(dir: String, hold_ms: f32) -> Self {
        let mut app = Self {
            dir: dir.clone(),
            hold_ms_str: format!("{:.0}", hold_ms),
            hold_ms,
            ..Default::default()
        };
        if !dir.is_empty() {
            app.load();
        }
        app
    }

    fn load(&mut self) {
        self.hold_ms = self.hold_ms_str.parse().unwrap_or(3000.0);
        let hold_bins = (self.hold_ms / WINDOW_MS) as usize;
        self.rows.clear();

        let dx_dir = format!("{}/dx100", self.dir);
        let sy_dir = format!("{}/synth", self.dir);

        let mut dx_files: Vec<_> = std::fs::read_dir(&dx_dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map_or(false, |x| x == "wav"))
                    .map(|e| e.path())
                    .collect()
            })
            .unwrap_or_default();
        dx_files.sort();

        if dx_files.is_empty() {
            self.status = format!("No WAV files found in {dx_dir}");
            return;
        }

        for (idx, dx_path) in dx_files.iter().enumerate() {
            let voice_num = idx + 1;
            let fname = dx_path.file_name().unwrap().to_string_lossy().to_string();
            let stem = dx_path.file_stem().unwrap().to_string_lossy().to_string();
            let name = stem.splitn(2, '_').nth(1).unwrap_or(&stem).to_string();
            let sy_path = format!("{sy_dir}/{fname}");

            match (
                load_rms_bins(&dx_path.to_string_lossy()),
                load_rms_bins(&sy_path),
            ) {
                (Some(dx_bins), Some(sy_bins)) => {
                    let dx_peak = dx_bins.iter().cloned().fold(0.0_f32, f32::max);
                    let sy_peak = sy_bins.iter().cloned().fold(0.0_f32, f32::max);
                    let dx_onset = find_onset(&dx_bins);
                    let sy_onset = find_onset(&sy_bins);
                    let dx_m = compute_metrics(&dx_bins, dx_onset, hold_bins);
                    let sy_m = compute_metrics(&sy_bins, sy_onset, hold_bins);
                    self.rows.push((
                        voice_num,
                        Some(VoiceRow {
                            name,
                            dx_bins,
                            sy_bins,
                            dx_onset,
                            sy_onset,
                            dx_peak,
                            sy_peak,
                            hold_bins,
                            dx_m,
                            sy_m,
                        }),
                    ));
                }
                _ => {
                    self.rows.push((voice_num, None));
                }
            }
        }
        self.status = format!("Loaded {} voices from \"{}\"", self.rows.len(), self.dir);
    }
}

impl eframe::App for EgViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // ── Toolbar ───────────────────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.label("Dir:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.dir)
                        .hint_text("out/eg_bank")
                        .desired_width(200.0),
                );
                ui.label("hold ms:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.hold_ms_str)
                        .desired_width(52.0),
                );
                if ui.button("Load").clicked() {
                    self.load();
                }
                ui.separator();
                ui.selectable_value(&mut self.view_mode, ViewMode::Overlay, "Overlay");
                ui.selectable_value(&mut self.view_mode, ViewMode::SideBySide, "Side-by-side");
                ui.separator();
                ui.label(RichText::new("■ HW").color(HW_COLOR).small());
                ui.label(RichText::new("■ SY").color(SY_COLOR).small());
                ui.add_space(8.0);
                ui.label(RichText::new("│ = note-off").color(NOFF_COLOR).small());
                if !self.status.is_empty() {
                    ui.separator();
                    ui.label(RichText::new(&self.status).color(Color32::GRAY).small());
                }
            });

            ui.separator();

            if self.rows.is_empty() {
                ui.add_space(20.0);
                ui.label("No data loaded.  Set a directory containing dx100/ and synth/ subdirs, then press Load.");
                return;
            }

            // ── Column headers ────────────────────────────────────────────────
            let name_w = 90.0_f32;
            let metrics_w = 210.0_f32;
            // Subtract fixed columns + inter-column spacing + vertical scrollbar allowance
            let wave_w = (ui.available_width() - name_w - metrics_w - 44.0).max(200.0);

            ui.horizontal(|ui| {
                ui.add_space(name_w);
                match self.view_mode {
                    ViewMode::Overlay => {
                        ui.add_space(wave_w * 0.5 - 30.0);
                        ui.label(RichText::new("Envelope  (HW / SY)").strong().small());
                        ui.add_space(wave_w * 0.5 - 80.0);
                    }
                    ViewMode::SideBySide => {
                        let hw = wave_w * 0.5 - 2.0;
                        ui.add_space(hw * 0.5 - 30.0);
                        ui.label(RichText::new("Hardware (DX100)").color(HW_COLOR).strong().small());
                        ui.add_space(hw * 0.5 - 20.0);
                        ui.add_space(hw * 0.5 - 20.0);
                        ui.label(RichText::new("Softsynth").color(SY_COLOR).strong().small());
                        ui.add_space(hw * 0.5 + 4.0);
                    }
                }
                ui.label(
                    RichText::new("atk90          d1l        rls50       rls90")
                        .small()
                        .weak(),
                );
            });

            // ── Voice rows ────────────────────────────────────────────────────
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let wave_h = self.wave_height;

                    for (voice_num, row_opt) in &self.rows {
                        ui.horizontal(|ui| {
                            // Name
                            ui.vertical(|ui| {
                                ui.set_width(name_w);
                                ui.set_min_height(wave_h);
                                ui.add_space(4.0);
                                match row_opt {
                                    Some(row) => {
                                        ui.label(
                                            RichText::new(format!("{:2}. {}", voice_num, row.name))
                                                .strong()
                                                .small(),
                                        );
                                    }
                                    None => {
                                        ui.label(
                                            RichText::new(format!("{:2}. (no file)", voice_num))
                                                .color(Color32::DARK_GRAY)
                                                .small(),
                                        );
                                    }
                                }
                            });

                            if let Some(row) = row_opt {
                                match self.view_mode {
                                    ViewMode::Overlay => {
                                        draw_overlay(ui, row, Vec2::new(wave_w, wave_h));
                                    }
                                    ViewMode::SideBySide => {
                                        let hw = wave_w * 0.5 - 2.0;
                                        draw_waveform(
                                            ui,
                                            &row.dx_bins,
                                            row.dx_onset,
                                            row.dx_peak,
                                            row.hold_bins,
                                            Vec2::new(hw, wave_h),
                                            HW_COLOR,
                                        );
                                        ui.add_space(4.0);
                                        draw_waveform(
                                            ui,
                                            &row.sy_bins,
                                            row.sy_onset,
                                            row.sy_peak,
                                            row.hold_bins,
                                            Vec2::new(hw, wave_h),
                                            SY_COLOR,
                                        );
                                    }
                                }

                                ui.add_space(6.0);
                                ui.vertical(|ui| {
                                    ui.set_width(metrics_w);
                                    ui.set_min_height(wave_h);
                                    metrics_grid(ui, &row.dx_m, &row.sy_m);
                                });
                            }
                        });

                        ui.add_space(1.0);
                        ui.separator();
                    }
                });
        });
    }
}

// ── Waveform drawing ──────────────────────────────────────────────────────────

fn draw_waveform(
    ui: &mut egui::Ui,
    bins: &[f32],
    onset: usize,
    peak: f32,
    hold_bins: usize,
    size: Vec2,
    color: Color32,
) {
    let (response, painter) = ui.allocate_painter(size, egui::Sense::hover());
    let rect = response.rect;
    painter.rect_filled(rect, 2.0, BG_COLOR);
    painter.rect_stroke(rect, 2.0, Stroke::new(1.0, BORDER_COLOR));

    let total = bins.len().saturating_sub(onset);
    if total == 0 || peak < 1e-7 {
        return;
    }

    waveform_inner(&painter, rect, bins, onset, peak, hold_bins, total, color);
}

fn draw_overlay(ui: &mut egui::Ui, row: &VoiceRow, size: Vec2) {
    let (response, painter) = ui.allocate_painter(size, egui::Sense::hover());
    let rect = response.rect;
    painter.rect_filled(rect, 2.0, BG_COLOR);
    painter.rect_stroke(rect, 2.0, Stroke::new(1.0, BORDER_COLOR));

    let dx_total = row.dx_bins.len().saturating_sub(row.dx_onset);
    let sy_total = row.sy_bins.len().saturating_sub(row.sy_onset);
    let total = dx_total.max(sy_total);
    if total == 0 {
        return;
    }

    waveform_inner(
        &painter,
        rect,
        &row.dx_bins,
        row.dx_onset,
        row.dx_peak,
        row.hold_bins,
        total,
        HW_COLOR,
    );
    waveform_inner(
        &painter,
        rect,
        &row.sy_bins,
        row.sy_onset,
        row.sy_peak,
        row.hold_bins,
        total,
        SY_COLOR,
    );

    // Small legend
    let font = egui::FontId::proportional(9.0);
    painter.text(
        rect.left_top() + Vec2::new(4.0, 2.0),
        egui::Align2::LEFT_TOP,
        "HW",
        font.clone(),
        HW_COLOR,
    );
    painter.text(
        rect.left_top() + Vec2::new(22.0, 2.0),
        egui::Align2::LEFT_TOP,
        "SY",
        font,
        SY_COLOR,
    );
}

fn waveform_inner(
    painter: &egui::Painter,
    rect: egui::Rect,
    bins: &[f32],
    onset: usize,
    peak: f32,
    hold_bins: usize,
    total: usize,
    color: Color32,
) {
    let margin_top = rect.height() * 0.05;
    let usable_h = rect.height() * 0.90;

    let to_x = |n: usize| rect.left() + (n as f32 / total as f32) * rect.width();
    let to_y = |v: f32| rect.top() + margin_top + (1.0 - (v / peak).clamp(0.0, 1.0)) * usable_h;

    // Note-off line
    let nx = to_x(hold_bins.min(total));
    painter.line_segment(
        [egui::pos2(nx, rect.top()), egui::pos2(nx, rect.bottom())],
        Stroke::new(1.0, NOFF_COLOR),
    );

    // Waveform polyline
    let bin_total = bins.len().saturating_sub(onset);
    let n_pts = bin_total.min(total);
    if n_pts < 2 {
        return;
    }
    let points: Vec<egui::Pos2> = (0..n_pts)
        .map(|n| {
            let v = bins[onset + n];
            egui::pos2(to_x(n), to_y(v))
        })
        .collect();
    painter.add(egui::Shape::line(points, Stroke::new(1.5, color)));
}

// ── Metrics display ───────────────────────────────────────────────────────────

fn match_color(hw: f32, sy: f32) -> Color32 {
    if hw.is_nan() && sy.is_nan() {
        return Color32::GRAY;
    }
    if hw.is_nan() || sy.is_nan() {
        return Color32::from_rgb(220, 80, 80);
    }
    // Both near zero (e.g. instant attack, decayed to silence): good match
    if hw < 1.0 && sy < 1.0 {
        // For ms values: both < 1ms → both instant → green
        // For level values (0..1): hw < 1.0 is always true, so use absolute diff
        let diff = (sy - hw).abs();
        if diff < 0.01 {
            return Color32::from_rgb(80, 200, 80);
        }
    }
    if hw.abs() < 1e-6 {
        return Color32::GRAY; // avoid divide-by-zero
    }
    let err = ((sy - hw) / hw).abs();
    if err < 0.15 {
        Color32::from_rgb(80, 200, 80)
    } else if err < 0.50 {
        Color32::from_rgb(220, 200, 60)
    } else {
        Color32::from_rgb(220, 80, 80)
    }
}

fn metrics_grid(ui: &mut egui::Ui, dx: &EgMetrics, sy: &EgMetrics) {
    let ms = |v: f32| -> String {
        if v.is_nan() {
            "   NaN  ".to_string()
        } else {
            format!("{:6.0}ms", v)
        }
    };
    let lv = |v: f32| -> String { format!("{:.3}", v) };

    egui::Grid::new(ui.next_auto_id())
        .num_columns(3)
        .min_col_width(50.0)
        .spacing([4.0, 2.0])
        .show(ui, |ui| {
            // atk90
            ui.label(RichText::new("atk90:").weak().small());
            ui.label(
                RichText::new(ms(dx.atk90_ms))
                    .color(HW_COLOR)
                    .monospace()
                    .small(),
            );
            ui.label(
                RichText::new(ms(sy.atk90_ms))
                    .color(match_color(dx.atk90_ms, sy.atk90_ms))
                    .monospace()
                    .small(),
            );
            ui.end_row();

            // d1l
            ui.label(RichText::new("d1l:  ").weak().small());
            ui.label(
                RichText::new(lv(dx.d1l))
                    .color(HW_COLOR)
                    .monospace()
                    .small(),
            );
            ui.label(
                RichText::new(lv(sy.d1l))
                    .color(match_color(dx.d1l, sy.d1l))
                    .monospace()
                    .small(),
            );
            ui.end_row();

            // rls50
            ui.label(RichText::new("rls50:").weak().small());
            ui.label(
                RichText::new(ms(dx.rls50_ms))
                    .color(HW_COLOR)
                    .monospace()
                    .small(),
            );
            ui.label(
                RichText::new(ms(sy.rls50_ms))
                    .color(match_color(dx.rls50_ms, sy.rls50_ms))
                    .monospace()
                    .small(),
            );
            ui.end_row();

            // rls90
            ui.label(RichText::new("rls90:").weak().small());
            ui.label(
                RichText::new(ms(dx.rls90_ms))
                    .color(HW_COLOR)
                    .monospace()
                    .small(),
            );
            ui.label(
                RichText::new(ms(sy.rls90_ms))
                    .color(match_color(dx.rls90_ms, sy.rls90_ms))
                    .monospace()
                    .small(),
            );
            ui.end_row();
        });
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = flag_val(&args, "--dir").unwrap_or_else(|| "out/eg_bank".to_string());
    let hold_ms: f32 = flag_val(&args, "--hold-ms")
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000.0);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("EG Compare Viewer")
            .with_inner_size([1200.0, 920.0]),
        ..Default::default()
    };

    eframe::run_native(
        "eg-viewer",
        options,
        Box::new(move |_cc| Ok(Box::new(EgViewerApp::new_with_args(dir.clone(), hold_ms)))),
    )
}

fn flag_val(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}
