//! Chart rendering shared by the PNG exporter and the interactive GUI.
//!
//! Each [`View`] has an `*_area` renderer that draws into a caller-provided
//! [`DArea`] (a plotters drawing area on a bitmap backend). The PNG exporter
//! wraps them with `BitMapBackend::new(path, size)`; the GUI renders into an
//! in-memory buffer with `BitMapBackend::with_buffer`.
pub mod cpu;
pub mod frame_time;
pub mod gpu;
pub mod histogram;
pub mod latency;
pub mod report;

use std::error::Error;
use std::ops::Add;

use plotters::coord::types::RangedCoordf32;
use plotters::prelude::*;

use crate::model::*;

/// A drawing area on a bitmap backend (either a PNG file or an in-memory buffer).
pub type DArea<'a> = DrawingArea<BitMapBackend<'a>, plotters::coord::Shift>;

/// Parameters that can be adjusted live from the GUI.
#[derive(Clone, Copy)]
pub struct RenderParams {
    /// Downsample bucket count for line series.
    pub bucket_num: usize,
    /// Histogram x-axis range: the percentile of frame times shown (50..100).
    pub hist_pct: f32,
    /// Use a log-scale y-axis on the histogram so the stutter tail stays visible.
    pub hist_log_y: bool,
    /// Y-axis padding, as a fraction of the data span added on each side of
    /// auto-scaled charts.
    pub y_pad: f32,
    /// Series line width in pixels.
    pub line_width: u32,
    /// Draw min/max annotations on charts.
    pub show_extremes: bool,
    /// Sweep animation progress. `None` = static full draw; `Some(p)` reveals
    /// the data up to `p` (0..1) of the time axis.
    pub anim_progress: Option<f32>,
}

impl Default for RenderParams {
    fn default() -> Self {
        RenderParams {
            bucket_num: BUCKET_NUM,
            hist_pct: 99.5,
            hist_log_y: false,
            y_pad: 0.08,
            line_width: 2,
            show_extremes: true,
            anim_progress: None,
        }
    }
}

/// Per-view canvas size (the frame-time chart is taller to fit the stats banner,
/// the report grows with its stutter-event list).
pub fn view_size(view: View, st: &Stats) -> (u32, u32) {
    match view {
        View::FrameTime => (IMG_W, IMG_H_TALL),
        View::Report => report::report_size(st),
        _ => (IMG_W, IMG_H),
    }
}

/// A time series plus its true extreme points (computed from raw data, so the
/// annotations stay accurate even though the drawn line is downsampled).
#[derive(Clone)]
pub struct Series {
    pub name: String,
    pub color: RGBColor,
    pub full: Vec<(f32, f32)>,
    pub min_pt: Option<(f32, f32)>,
    pub max_pt: Option<(f32, f32)>,
}

#[derive(Clone, Copy)]
enum BucketMode {
    Avg,
    Max,
}

/// Reduces `points` to at most `bucket_num` buckets along the x axis.
fn downsample(points: &[(f32, f32)], bucket_num: usize, mode: BucketMode) -> Vec<(f32, f32)> {
    if points.len() <= bucket_num || points.is_empty() {
        return points.to_vec();
    }
    let x0 = points[0].0;
    let x1 = points[points.len() - 1].0;
    let span = (x1 - x0).max(1e-6);

    let mut out: Vec<(f32, f32)> = Vec::with_capacity(bucket_num);
    let mut start = 0usize;
    for b in 0..bucket_num {
        if start >= points.len() {
            break;
        }
        let right = x0 + span * ((b + 1) as f32 / bucket_num as f32);
        let mut end = start;
        while end < points.len() && points[end].0 < right {
            end += 1;
        }
        let end = end.max(start + 1).min(points.len());

        let v = match mode {
            BucketMode::Avg => {
                let mut s: f32 = 0.0;
                for i in start..end {
                    s += points[i].1;
                }
                s / (end - start) as f32
            }
            BucketMode::Max => {
                let mut m = f32::MIN;
                for i in start..end {
                    if points[i].1 > m {
                        m = points[i].1;
                    }
                }
                m
            }
        };
        out.push(((points[start].0 + points[end - 1].0) * 0.5, v));
        start = end;
    }
    out
}

/// Clamps every point into the visible axis range. This is the defense-in-depth
/// that keeps plotters from ever seeing a value so far outside the range that
/// its integer coordinate math would overflow (see the crash fixed here).
pub fn clamp_pts(pts: &[(f32, f32)], x0: f32, x1: f32, y0: f32, y1: f32) -> Vec<(f32, f32)> {
    let (lo_x, hi_x) = (x0.min(x1), x0.max(x1));
    let (lo_y, hi_y) = (y0.min(y1), y0.max(y1));
    pts.iter()
        .map(|&(x, y)| (x.clamp(lo_x, hi_x), y.clamp(lo_y, hi_y)))
        .collect()
}

/// Builds a series from one column of the capture, recording true min/max points.
pub fn make_series<F: Fn(&FrameSample) -> f32>(
    sess: &Session,
    select: F,
    color: RGBColor,
    name: &str,
) -> Series {
    let mut full: Vec<(f32, f32)> = Vec::with_capacity(sess.frames.len());
    let mut min_pt: Option<(f32, f32)> = None;
    let mut max_pt: Option<(f32, f32)> = None;
    for f in &sess.frames {
        let v = select(f);
        if v.is_finite() {
            let p = (f.time_s, v);
            full.push(p);
            if min_pt.map_or(true, |m| v < m.1) {
                min_pt = Some(p);
            }
            if max_pt.map_or(true, |m| v > m.1) {
                max_pt = Some(p);
            }
        }
    }
    Series {
        name: name.to_string(),
        color,
        full,
        min_pt,
        max_pt,
    }
}

/// Minimum and maximum finite value across a set of series.
pub fn series_minmax(list: &[&Series]) -> Option<(f32, f32)> {
    let mut lo = f32::MAX;
    let mut hi = f32::MIN;
    let mut any = false;
    for s in list {
        for &(_, v) in &s.full {
            if v.is_finite() {
                any = true;
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
    }
    if any {
        Some((lo, hi))
    } else {
        None
    }
}

/// A data-driven y range with `pad` * span of padding on each side.
pub fn padded_range(list: &[&Series], pad: f32) -> (f32, f32) {
    match series_minmax(list) {
        Some((lo, hi)) if hi > lo => {
            let span = hi - lo;
            (lo - span * pad, hi + span * pad)
        }
        Some((v, _)) => (v - 1.0, v + 1.0),
        None => (0.0, 1.0),
    }
}

/// A distinct color for the `i`-th of `n` items (hue-spaced, for CPU cores).
pub fn core_color(i: usize, n: usize) -> RGBColor {
    let h = i as f64 * 360.0 / n.max(1) as f64;
    hsl_to_rgb(h, 0.72, 0.45)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> RGBColor {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match (hp.floor() as i32).rem_euclid(6) {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    RGBColor(
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    )
}

/// Options controlling how a panel is drawn (per-series legend / annotations).
#[derive(Clone, Copy)]
pub struct PanelOptions {
    pub legend: bool,
    pub annotate: bool,
}

impl Default for PanelOptions {
    fn default() -> Self {
        PanelOptions {
            legend: true,
            annotate: true,
        }
    }
}

/// When the sweep animation is active, the x value up to which data is revealed.
pub fn sweep_cut(params: &RenderParams, x0: f32, x1: f32) -> Option<f32> {
    match params.anim_progress {
        Some(p) if p < 1.0 => Some(x0 + (x1 - x0) * p.clamp(0.0, 1.0)),
        _ => None,
    }
}

/// The points to draw for a series, optionally clipped to the sweep cursor and
/// then downsampled.
pub fn series_pts(s: &Series, cut: Option<f32>, bucket_num: usize) -> Vec<(f32, f32)> {
    match cut {
        Some(c) => {
            let clipped: Vec<(f32, f32)> = s.full.iter().copied().filter(|p| p.0 <= c).collect();
            downsample(&clipped, bucket_num, BucketMode::Avg)
        }
        None => downsample(&s.full, bucket_num, BucketMode::Avg),
    }
}

/// Draws text at an absolute pixel position on the drawing area.
pub fn draw_text(area: &DArea<'_>, text: &str, x: i32, y: i32, size: u32) -> Result<(), Box<dyn Error>> {
    let style = TextStyle::from(("sans-serif", size)).color(&C_CAPTION);
    area.draw_text(text, &style, (x, y))?;
    Ok(())
}

/// Draws a single text label at a data point (logical offset), e.g. `max 613`.
pub fn draw_label_at<X, Y>(
    chart: &mut ChartContext<'_, BitMapBackend<'_>, Cartesian2d<X, Y>>,
    point: (X::ValueType, Y::ValueType),
    offset: (X::ValueType, Y::ValueType),
    text: String,
    color: RGBColor,
) -> Result<(), Box<dyn Error>>
where
    X: Ranged,
    Y: Ranged,
    X::ValueType: Copy + Add<X::ValueType, Output = X::ValueType> + 'static,
    Y::ValueType: Copy + Add<Y::ValueType, Output = Y::ValueType> + 'static,
{
    let coord = (point.0 + offset.0, point.1 + offset.1);
    let style = TextStyle::from(("sans-serif", 12)).color(&color);
    chart.draw_series(std::iter::once(Text::new(text, coord, style)))?;
    Ok(())
}

/// Draws a panel chart with a light grid, legend, optional min/max annotations,
/// and (when animating) a sweep cursor.
pub fn draw_panel(
    area: &DArea<'_>,
    title: &str,
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,
    list: &[&Series],
    params: &RenderParams,
    opts: PanelOptions,
) -> Result<(), Box<dyn Error>> {
    let mut chart = ChartBuilder::on(area)
        .caption(title, TextStyle::from(("sans-serif", 20)).color(&C_CAPTION))
        .margin(8)
        .x_label_area_size(24)
        .y_label_area_size(38)
        .build_cartesian_2d(x0..x1, y0..y1)?;

    chart
        .configure_mesh()
        .light_line_style(C_GRID_LIGHT)
        .bold_line_style(C_GRID_BOLD)
        .label_style(TextStyle::from(("sans-serif", 12)).color(&C_AXIS_TEXT))
        .x_labels(8)
        .draw()?;

    let cut = sweep_cut(params, x0, x1);
    let x_range = x1 - x0;
    for (idx, item) in list.iter().enumerate() {
        if item.full.is_empty() {
            continue;
        }
        let pts = clamp_pts(&series_pts(item, cut, params.bucket_num), x0, x1, y0, y1);
        if pts.is_empty() {
            continue;
        }
        let anno = chart.draw_series(LineSeries::new(pts, item.color.stroke_width(params.line_width)))?;
        if opts.legend {
            // The colored line swatch in the legend only appears if we also
            // register a `.legend()` draw function (`.label()` alone yields
            // plain text with no color icon).
            let swatch = item.color.stroke_width(3);
            anno.label(&item.name)
                .legend(move |p: (i32, i32)| PathElement::new(vec![(p.0, p.1), (p.0 + 20, p.1)], swatch));
        }
        if opts.annotate && params.show_extremes && cut.is_none() {
            annotate_series(&mut chart, item, idx, x_range)?;
        }
    }

    if let Some(c) = cut {
        if c > x0 && c < x1 {
            chart.draw_series(LineSeries::new(
                [(c, y0), (c, y1)],
                RGBColor(90, 90, 90).mix(0.5).stroke_width(1),
            ))?;
        }
    }

    if opts.legend {
        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperLeft)
            .background_style(RGBAColor(255, 255, 255, 0.9))
            .border_style(C_GRID_BOLD)
            .label_font(TextStyle::from(("sans-serif", 12)).color(&C_AXIS_TEXT))
            .draw()?;
    }
    Ok(())
}

/// Marks a series' lowest and highest points and labels them with their values.
/// Labels of the second series are nudged vertically so GPU0/GPU1 don't overlap.
fn annotate_series(
    chart: &mut ChartContext<'_, BitMapBackend<'_>, Cartesian2d<RangedCoordf32, RangedCoordf32>>,
    series: &Series,
    series_idx: usize,
    x_range: f32,
) -> Result<(), Box<dyn Error>> {
    let dx = x_range * 0.03;
    let dy = series_idx as f32 * 2.5;

    let mut markers: Vec<(f32, f32)> = Vec::new();
    if let Some(p) = series.min_pt {
        markers.push(p);
    }
    if let Some(p) = series.max_pt {
        markers.push(p);
    }
    if !markers.is_empty() {
        chart.draw_series(PointSeries::of_element(
            markers,
            5,
            series.color.filled(),
            &|c, s, st| EmptyElement::at(c) + Circle::new((0, 0), s, st),
        ))?;
    }

    if let Some(p) = series.min_pt {
        draw_label_at(chart, p, (-dx, -dy), format!("min {:.1}", p.1), series.color)?;
    }
    if let Some(p) = series.max_pt {
        draw_label_at(chart, p, (dx, dy), format!("max {:.1}", p.1), series.color)?;
    }
    Ok(())
}

/// Renders one view into a caller-provided drawing area.
pub fn render_view(
    view: View,
    sess: &Session,
    st: &Stats,
    params: &RenderParams,
    area: &DArea<'_>,
) -> Result<(), Box<dyn Error>> {
    match view {
        View::FrameTime => frame_time::render_frame_time_area(sess, st, params, area),
        View::Report => report::render_report_area(sess, st, area),
        View::Histogram => histogram::render_histogram_area(sess, st, params, area),
        View::Gpu => gpu::render_gpu_area(sess, st, params, area),
        View::Cpu => cpu::render_cpu_area(sess, st, params, area),
        View::Latency => latency::render_latency_area(sess, st, params, area),
    }
}

/// Renders one view into an in-memory RGB buffer (used by the GUI).
pub fn render_view_to_buffer(
    view: View,
    sess: &Session,
    st: &Stats,
    params: &RenderParams,
    size: (u32, u32),
) -> Result<Vec<u8>, Box<dyn Error>> {
    let (w, h) = size;
    let mut buf = vec![0u8; (w as usize) * (h as usize) * 3];
    {
        let backend = BitMapBackend::with_buffer(&mut buf, size);
        let area = backend.into_drawing_area();
        area.fill(&WHITE)?;
        render_view(view, sess, st, params, &area)?;
        area.present()?;
    }
    Ok(buf)
}
