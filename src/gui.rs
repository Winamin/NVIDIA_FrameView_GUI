//! Interactive chart viewer built on `eframe`/`egui` (the same UI stack whiskers
//! uses), rendering the plotters charts into an in-memory buffer each frame.
//!
//! Provides: view switching, live parameters (bucket count / histogram bin width
//! / y padding / annotations), a time-sweep animation, mouse-wheel zoom with
//! panning (re-rendered at the zoomed resolution for crispness), and PNG export.
use std::time::Instant;

use eframe::egui;

use crate::csv::load_csv;
use crate::export::render_view_to_file;
use crate::model::*;
use crate::render::{render_view_to_buffer, view_size, RenderParams};
use crate::stats::analyze;

pub struct ChartApp {
    sess: Session,
    st: Stats,
    view: View,
    // render parameters
    bucket_num: usize,
    hist_pct: f32,
    hist_log_y: bool,
    y_pad: f32,
    line_width: u32,
    show_extremes: bool,
    // animation
    anim_playing: bool,
    anim_speed: f32,
    anim_progress: f32,
    last_tick: Instant,
    // zoom / pan
    zoom: f32,
    pan: egui::Vec2,
    last_render_size: (u32, u32),
    // display
    texture: Option<egui::TextureHandle>,
    needs_redraw: bool,
    status: String,
}

impl ChartApp {
    pub fn new(sess: Session, st: Stats) -> Self {
        Self {
            sess,
            st,
            view: View::FrameTime,
            bucket_num: BUCKET_NUM,
            hist_pct: 99.5,
            hist_log_y: false,
            y_pad: 0.08,
            line_width: 2,
            show_extremes: true,
            anim_playing: false,
            anim_speed: 0.25,
            anim_progress: 1.0,
            last_tick: Instant::now(),
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            last_render_size: (0, 0),
            texture: None,
            needs_redraw: true,
            status: String::new(),
        }
    }

    fn render_params(&self) -> RenderParams {
        RenderParams {
            bucket_num: self.bucket_num,
            hist_pct: self.hist_pct,
            hist_log_y: self.hist_log_y,
            y_pad: self.y_pad,
            line_width: self.line_width,
            show_extremes: self.show_extremes,
            // progress < 1.0 = sweep reveal; 1.0 (or idle) = full draw.
            anim_progress: if self.anim_progress < 1.0 {
                Some(self.anim_progress)
            } else {
                None
            },
        }
    }

    fn render_texture(&mut self, ctx: &egui::Context, size: (u32, u32)) {
        if size.0 == 0 || size.1 == 0 {
            return;
        }
        let params = self.render_params();
        match render_view_to_buffer(self.view, &self.sess, &self.st, &params, size) {
            Ok(buf) => {
                let color = egui::ColorImage::from_rgb([size.0 as usize, size.1 as usize], &buf);
                self.texture = Some(ctx.load_texture("chart", color, egui::TextureOptions::default()));
                self.needs_redraw = false;
            }
            Err(e) => {
                self.status = format!("Render error: {e}");
            }
        }
    }

    fn save_png(&mut self) {
        let params = self.render_params();
        match render_view_to_file(self.view, &self.sess, &self.st, &params, OUT_DIR) {
            Ok(path) => self.status = format!("Saved {}", path.display()),
            Err(e) => self.status = format!("Save failed: {e}"),
        }
    }

    fn load_new_csv(&mut self) {
        let Some(file) = rfd::FileDialog::new()
            .add_filter("CSV files", &["csv"])
            .add_filter("All files", &["*"])
            .pick_file()
        else {
            return;
        };
        let path = file.to_string_lossy().to_string();
        let mut sess = Session {
            app: String::new(),
            gpu: String::new(),
            cpu: String::new(),
            resolution: String::new(),
            runtime: String::new(),
            frames: Vec::new(),
        };
        match load_csv(&path, &mut sess) {
            Ok(()) => {
                let mut st = Stats::default();
                analyze(&sess, &mut st);
                self.sess = sess;
                self.st = st;
                self.texture = None;
                self.needs_redraw = true;
                self.anim_playing = false;
                self.anim_progress = 1.0;
                self.zoom = 1.0;
                self.pan = egui::Vec2::ZERO;
                self.status = format!("Loaded {path}");
            }
            Err(e) => self.status = format!("Failed to load {path}: {e}"),
        }
    }

    fn controls_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.vertical_centered(|ui| {
            ui.heading("FrameView Charts");
        });
        ui.label(
            egui::RichText::new(format!(
                "{} · {} frames · avg {:.2} ms · {:.0} fps",
                self.sess.app, self.st.frame_num, self.st.avg_ms, self.st.avg_fps
            ))
            .size(12.0)
            .color(egui::Color32::from_gray(150)),
        );
        ui.separator();

        ui.add_space(4.0);
        ui.strong("Views");
        for v in View::ALL {
            if ui.selectable_value(&mut self.view, v, v.label()).changed() {
                self.needs_redraw = true;
                self.pan = egui::Vec2::ZERO;
                self.status.clear();
            }
        }

        ui.separator();
        ui.strong("Parameters");
        if ui
            .add(egui::Slider::new(&mut self.bucket_num, 200..=5000).text("downsample buckets"))
            .changed()
        {
            self.needs_redraw = true;
        }
        if ui
            .add(egui::Slider::new(&mut self.hist_pct, 95.0..=100.0).text("histogram range (%)"))
            .changed()
        {
            self.needs_redraw = true;
        }
        if ui.checkbox(&mut self.hist_log_y, "Histogram log-scale count axis").changed() {
            self.needs_redraw = true;
        }
        if ui
            .add(egui::Slider::new(&mut self.y_pad, 0.0..=0.5).text("y padding"))
            .changed()
        {
            self.needs_redraw = true;
        }
        if ui
            .add(egui::Slider::new(&mut self.line_width, 1..=4).text("line width (px)"))
            .changed()
        {
            self.needs_redraw = true;
        }
        if ui.checkbox(&mut self.show_extremes, "Show min / max labels").changed() {
            self.needs_redraw = true;
        }

        ui.separator();
        ui.strong("Animation");
        let play_label = if self.anim_playing { "⏸  Pause" } else { "▶  Play sweep" };
        if ui.button(play_label).clicked() {
            self.anim_playing = !self.anim_playing;
            if self.anim_playing && self.anim_progress >= 1.0 {
                self.anim_progress = 0.0;
            }
            self.last_tick = Instant::now();
            self.needs_redraw = true;
            if self.anim_playing {
                ui.ctx().request_repaint();
            }
        }
        ui.add(egui::Slider::new(&mut self.anim_speed, 0.05..=2.0).text("sweep speed"));
        if ui
            .add(egui::Slider::new(&mut self.anim_progress, 0.0..=1.0).text("progress"))
            .changed()
        {
            self.anim_playing = false;
            self.needs_redraw = true;
        }

        ui.separator();
        ui.strong("Zoom");
        ui.horizontal(|ui| {
            if ui.button("−").clicked() {
                self.zoom = (self.zoom / 1.25).max(1.0);
                self.needs_redraw = true;
            }
            if ui.button("1:1 fit").clicked() {
                self.zoom = 1.0;
                self.pan = egui::Vec2::ZERO;
                self.needs_redraw = true;
            }
            if ui.button("+").clicked() {
                self.zoom = (self.zoom * 1.25).min(8.0);
                self.needs_redraw = true;
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
        });
        ui.label(
            egui::RichText::new("wheel = zoom · drag = pan")
                .size(11.0)
                .color(egui::Color32::from_gray(140)),
        );

        ui.separator();
        ui.strong("Export");
        if ui.button("Save current chart as PNG").clicked() {
            self.save_png();
        }
        if ui.button("Load another CSV…").clicked() {
            self.load_new_csv();
        }

        if !self.status.is_empty() {
            ui.separator();
            ui.label(&self.status);
        }
    }

    fn chart_ui(&mut self, ui: &mut egui::Ui) {
        let max_side = ui.ctx().input(|i| i.max_texture_side) as f32;
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

        /* Mouse-wheel zoom around the cursor. */
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            if let Some(pointer) = resp.hover_pos() {
                if rect.contains(pointer) {
                    let factor = (scroll * 0.01).exp();
                    let new_zoom = (self.zoom * factor).clamp(1.0, 8.0);
                    if (new_zoom - self.zoom).abs() > 1e-4 {
                        self.zoom = new_zoom;
                        self.needs_redraw = true;
                    }
                }
            }
        }
        if resp.dragged() {
            self.pan += resp.drag_delta();
        }
        if self.zoom <= 1.001 {
            self.pan = egui::Vec2::ZERO;
        }

        /* Render at the zoomed resolution (crisp when magnified). */
        let base = view_size(self.view, &self.st);
        let fit = (rect.width() / base.0 as f32)
            .min(rect.height() / base.1 as f32)
            .max(0.01);
        let display = egui::vec2(base.0 as f32 * fit, base.1 as f32 * fit) * self.zoom;
        let ppp = ui.ctx().pixels_per_point();
        let render = clamp_to_texture(display * ppp, max_side);

        if self.needs_redraw || self.texture.is_none() || render != self.last_render_size {
            self.render_texture(ui.ctx(), render);
            self.last_render_size = render;
        }

        let Some(tex) = &self.texture else {
            ui.label(&self.status);
            return;
        };

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(28, 30, 36));
        let origin = rect.center() - display * 0.5 + self.pan;
        let img_rect = egui::Rect::from_min_size(origin, display);
        painter.image(
            tex.id(),
            img_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        painter.text(
            rect.right_bottom() + egui::vec2(-10.0, -6.0),
            egui::Align2::RIGHT_BOTTOM,
            format!("{:.0}%  ·  wheel: zoom · drag: pan", self.zoom * 100.0),
            egui::FontId::proportional(13.0),
            egui::Color32::from_gray(180),
        );
    }
}

/// Clamps a pixel size so both sides stay within the GPU texture limit.
fn clamp_to_texture(size: egui::Vec2, max_side: f32) -> (u32, u32) {
    if size.x <= 0.0 || size.y <= 0.0 || max_side <= 0.0 {
        return (0, 0);
    }
    let s = (max_side / size.x).min(max_side / size.y).min(1.0);
    ((size.x * s).round() as u32, (size.y * s).round() as u32)
}

impl eframe::App for ChartApp {
    /// Advance the sweep animation; keeps repainting while playing.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.anim_playing {
            let dt = self.last_tick.elapsed().as_secs_f32();
            self.last_tick = Instant::now();
            self.anim_progress += dt * self.anim_speed;
            if self.anim_progress >= 1.0 {
                self.anim_progress = 0.0; // loop
            }
            self.needs_redraw = true;
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::left("controls")
            .min_size(230.0)
            .max_size(320.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.controls_ui(ui);
                });
            });
        egui::CentralPanel::default().show(ui, |ui| {
            self.chart_ui(ui);
        });
    }
}

/// Launches the interactive viewer. `sess`/`st` are moved in.
pub fn run(sess: Session, st: Stats) -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1360.0, 880.0])
            .with_title("FrameView Chart Viewer"),
        ..Default::default()
    };
    eframe::run_native(
        "NVIDIA FrameView GUI",
        native_options,
        Box::new(move |_cc| Ok(Box::new(ChartApp::new(sess, st)))),
    )
}
