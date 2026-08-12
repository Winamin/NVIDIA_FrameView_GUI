//! Frame-time distribution histogram.
//!
//! The x-axis is data-driven: it spans from the 0.5th percentile up to the
//! `hist_pct`-th percentile of frame times (with a little padding), so the bars
//! fill the chart instead of collapsing into one bar on a fixed 0..120 ms axis.
//! The slowest frames (beyond the shown range) land in a red overflow bar at the
//! right edge; the fastest 0.5% are folded into the first bin.
use std::error::Error;

use plotters::coord::ranged1d::ValueFormatter;
use plotters::coord::types::RangedCoordi32;
use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};

use crate::model::*;
use super::{DArea, RenderParams};

const BIN_N: usize = 60;

pub fn render_histogram_area(
    sess: &Session,
    st: &Stats,
    params: &RenderParams,
    area: &DArea<'_>,
) -> Result<(), Box<dyn Error>> {
    /* Collect and sort frame times. */
    let mut times: Vec<f32> = Vec::with_capacity(sess.frames.len());
    for f in &sess.frames {
        let v = f.frame_ms;
        if v.is_finite() && v > 0.0 {
            times.push(v);
        }
    }
    if times.is_empty() {
        return Ok(());
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());

    /* Data-driven x range: [p0.5, p(hist_pct)], padded a little. */
    let n = times.len();
    let pct = params.hist_pct.clamp(50.0, 100.0) / 100.0;
    let hi_idx = ((n - 1) as f32 * pct).round() as usize;
    let lo_idx = ((n - 1) as f32 * 0.005).round() as usize;
    let mut x_lo = times[lo_idx];
    let mut x_hi = times[hi_idx];
    if x_hi - x_lo < 1e-3 {
        x_lo = (x_lo - 1.0).max(0.0);
        x_hi = x_hi + 1.0;
    }
    let pad = (x_hi - x_lo) * 0.04;
    x_lo = (x_lo - pad).max(0.0);
    x_hi += pad;
    let bin_w = (x_hi - x_lo) / BIN_N as f32;

    let mut bins = vec![0u64; BIN_N];
    let mut overflow: u64 = 0;
    for &v in &times {
        if v >= x_hi {
            overflow += 1;
        } else if v >= x_lo {
            bins[((v - x_lo) / bin_w) as usize] += 1;
        } else {
            bins[0] += 1; // the fastest 0.5% tail
        }
    }

    let mut maxc: u64 = 1;
    let mut peak_idx = 0usize;
    let mut peak_count = 0u64;
    for (i, &b) in bins.iter().enumerate() {
        if b > maxc {
            maxc = b;
        }
        if b > peak_count {
            peak_count = b;
            peak_idx = i;
        }
    }
    if overflow > maxc {
        maxc = overflow;
    }

    /* Sweep animation: bars grow from 0 to full height as progress advances. */
    let anim_p = params.anim_progress.unwrap_or(1.0).clamp(0.0, 1.0);
    let grow = |c: u64| c as f64 * anim_p as f64;

    let data: Vec<(i32, f64)> = bins
        .iter()
        .enumerate()
        .map(|(i, &c)| (i as i32, grow(c)))
        .collect();
    let overflow_data: Vec<(i32, f64)> = vec![(BIN_N as i32, grow(overflow))];

    let maxc = maxc as f64;
    let y_max = maxc * (1.0 + params.y_pad as f64);

    let caption = TextStyle::from(("sans-serif", 30)).color(&C_CAPTION);
    let x_range = 0..(BIN_N as i32 + 1);

    if params.hist_log_y {
        /* Log y-axis: baseline at 1 so small tail bins stay visible. */
        let mut chart = ChartBuilder::on(area)
            .caption("Frame Time Distribution", caption)
            .margin(25)
            .x_label_area_size(60)
            .y_label_area_size(70)
            .build_cartesian_2d(x_range, (1.0..y_max).log_scale())?;
        draw_histogram_body(&mut chart, st, params, x_lo, x_hi, bin_w, &data, overflow, &overflow_data, peak_idx as i32, peak_count as f64, maxc)?;
    } else {
        let mut chart = ChartBuilder::on(area)
            .caption("Frame Time Distribution", caption)
            .margin(25)
            .x_label_area_size(60)
            .y_label_area_size(70)
            .build_cartesian_2d(x_range, 0.0..y_max)?;
        draw_histogram_body(&mut chart, st, params, x_lo, x_hi, bin_w, &data, overflow, &overflow_data, peak_idx as i32, peak_count as f64, maxc)?;
    }

    Ok(())
}

/// Draws the mesh, bars, avg line and annotations for a linear or log y-axis.
#[allow(clippy::too_many_arguments)]
fn draw_histogram_body<'a, Y: Ranged<ValueType = f64> + ValueFormatter<f64>>(
    chart: &mut ChartContext<'a, BitMapBackend<'a>, Cartesian2d<RangedCoordi32, Y>>,
    st: &Stats,
    params: &RenderParams,
    x_lo: f32,
    x_hi: f32,
    bin_w: f32,
    data: &[(i32, f64)],
    overflow: u64,
    overflow_data: &[(i32, f64)],
    peak_idx: i32,
    peak_count: f64,
    maxc: f64,
) -> Result<(), Box<dyn Error>> {
    chart
        .configure_mesh()
        .x_desc("Frame time (ms)")
        .y_desc("Frame count")
        .axis_desc_style(TextStyle::from(("sans-serif", 18)).color(&C_AXIS_TEXT))
        .label_style(TextStyle::from(("sans-serif", 16)).color(&C_AXIS_TEXT))
        .light_line_style(C_GRID_LIGHT)
        .bold_line_style(C_GRID_BOLD)
        .x_label_formatter(&|x: &i32| {
            let ms = x_lo + *x as f32 * bin_w;
            if bin_w >= 1.0 {
                format!("{ms:.0}")
            } else if bin_w >= 0.1 {
                format!("{ms:.1}")
            } else {
                format!("{ms:.2}")
            }
        })
        .x_labels(16)
        .draw()?;

    chart
        .draw_series(
            Histogram::vertical(chart)
                .style(C_SERIES_A.mix(0.55).filled())
                .data(data.iter().copied()),
        )?
        .label(format!("{x_lo:.1}-{x_hi:.1} ms"))
        .legend(|p: (i32, i32)| Rectangle::new([(p.0, p.1 - 5), (p.0 + 10, p.1 + 5)], C_SERIES_A.mix(0.55).filled()));

    if overflow > 0 {
        chart
            .draw_series(
                Histogram::vertical(chart)
                    .style(C_STUTTER.mix(0.7).filled())
                    .data(overflow_data.iter().copied()),
            )?
            .label(format!("> {x_hi:.1} ms"))
            .legend(|p: (i32, i32)| Rectangle::new([(p.0, p.1 - 5), (p.0 + 10, p.1 + 5)], C_STUTTER.mix(0.7).filled()));
    }

    if st.avg_ms > 0.0 {
        /* Clamp keeps the avg line inside the visible bins (crash guard). */
        let xbin = (((st.avg_ms - x_lo) / bin_w) as i32).clamp(0, BIN_N as i32);
        chart
            .draw_series(LineSeries::new(
                [(xbin, 0.0f64), (xbin, maxc)],
                C_AVG.stroke_width(3),
            ))?
            .label(format!("avg {:.2} ms", st.avg_ms))
            .legend(|p: (i32, i32)| PathElement::new(vec![(p.0, p.1), (p.0 + 20, p.1)], C_AVG.stroke_width(3)));
    }

    /* Leader-line annotations: a short colored line from each interesting bar's
       top with its value labelled just above the line (pixel offset, so it
       never collides). The blue peak extends RIGHT, the red overflow extends
       LEFT and the green avg extends LEFT, keeping the labels apart. */
    if params.show_extremes && params.anim_progress.is_none() {
        /* The leader line starts right at the bar's top (the bar height). The
           count sits at the FAR end of the line: blue peak extends RIGHT with
           its number on the right, red overflow extends LEFT with its number
           on the left. */
        let right_end = Pos::new(HPos::Left, VPos::Center);
        let left_end = Pos::new(HPos::Right, VPos::Center);

        /* Blue: peak bar count, number at the far right end. */
        if peak_count > 0.0 {
            let x1 = (peak_idx + 5).min(BIN_N as i32);
            let color = C_SERIES_A;
            let style = TextStyle::from(("sans-serif", 12)).color(&color).pos(right_end);
            chart.draw_series(LineSeries::new(
                [(peak_idx, peak_count), (x1, peak_count)],
                color.stroke_width(1),
            ))?;
            let text = format!("peak {}", peak_count as u64);
            chart.draw_series(PointSeries::of_element(
                vec![(x1, peak_count)],
                0,
                &color,
                &move |c, _s, _st| EmptyElement::at(c) + Text::new(text.clone(), (2, 0), style.clone()),
            ))?;
        }

        /* Red: overflow bar count, number at the far left end. */
        if overflow > 0 {
            let oc = overflow as f64;
            let x1 = (BIN_N as i32 - 5).max(0);
            let color = C_STUTTER;
            let style = TextStyle::from(("sans-serif", 12)).color(&color).pos(left_end);
            chart.draw_series(LineSeries::new(
                [(BIN_N as i32, oc), (x1, oc)],
                color.stroke_width(1),
            ))?;
            let text = format!("> {x_hi:.1} ms: {}", overflow);
            chart.draw_series(PointSeries::of_element(
                vec![(x1, oc)],
                0,
                &color,
                &move |c, _s, _st| EmptyElement::at(c) + Text::new(text.clone(), (-2, 0), style.clone()),
            ))?;
        }
    }

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperRight)
        .background_style(RGBAColor(255, 255, 255, 0.9))
        .border_style(C_GRID_BOLD)
        .label_font(TextStyle::from(("sans-serif", 16)).color(&C_AXIS_TEXT))
        .draw()?;

    Ok(())
}
