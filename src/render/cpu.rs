//! CPU monitor: a full-width per-core utilization panel (every core in its own
//! color) above the aggregate utilization / clock / temperature / power panels.
use std::error::Error;

use crate::model::*;
use super::{core_color, draw_panel, make_series, padded_range, DArea, PanelOptions, RenderParams, Series};

pub fn render_cpu_area(
    sess: &Session,
    _st: &Stats,
    params: &RenderParams,
    area: &DArea<'_>,
) -> Result<(), Box<dyn Error>> {
    let t0 = sess.frames[0].time_s;
    let t1 = sess.frames[sess.frames.len() - 1].time_s;

    /* How many cores actually have data in this capture. */
    let mut cores = 0usize;
    for f in &sess.frames {
        for c in 0..MAX_CORES {
            if f.cpu_core_util[c].is_finite() {
                cores = cores.max(c + 1);
            }
        }
    }
    if cores == 0 {
        cores = MAX_CORES;
    }

    /* One series per core, each a different hue. */
    let mut core_series: Vec<Series> = Vec::with_capacity(cores);
    for c in 0..cores {
        let color = core_color(c, cores);
        core_series.push(make_series(
            sess,
            move |f: &FrameSample| f.cpu_core_util[c],
            color,
            &format!("Core {c}"),
        ));
    }

    let util = make_series(sess, |f: &FrameSample| f.cpu_util, C_SERIES_A, "Util");
    let clk = make_series(sess, |f: &FrameSample| f.cpu_clk, C_SERIES_B, "Clk");
    let temp = make_series(sess, |f: &FrameSample| f.cpu_temp, C_SERIES_A, "Temp");
    let pwr = make_series(sess, |f: &FrameSample| f.cpu_power, C_SERIES_B, "Pkg Pwr");

    let opts = PanelOptions::default();
    // Every core gets a colored legend swatch so its line color is identifiable.
    // (With very many cores the legend grows; cap it to keep the panel usable.)
    let opts_cores = PanelOptions {
        legend: cores <= 24,
        annotate: false,
    };

    /* Full-width per-core panel on top, four panels below. */
    let core_refs: Vec<&Series> = core_series.iter().collect();
    let (top, bottom) = area.split_vertically(area.dim_in_pixel().1 as i32 / 3);
    let (y0, y1) = padded_range(&core_refs, params.y_pad);
    draw_panel(&top, "Per-Core CPU Utilization (%)", t0, t1, y0, y1, &core_refs, params, opts_cores)?;

    let areas = bottom.split_evenly((2, 2));
    let (y0, y1) = padded_range(&[&util], params.y_pad);
    draw_panel(&areas[0], "CPU Utilization (%)", t0, t1, y0, y1, &[&util], params, opts)?;
    let (y0, y1) = padded_range(&[&clk], params.y_pad);
    draw_panel(&areas[1], "CPU Clock (MHz)", t0, t1, y0, y1, &[&clk], params, opts)?;
    let (y0, y1) = padded_range(&[&temp], params.y_pad);
    draw_panel(&areas[2], "CPU Package Temp (C)", t0, t1, y0, y1, &[&temp], params, opts)?;
    let (y0, y1) = padded_range(&[&pwr], params.y_pad);
    draw_panel(&areas[3], "CPU Package Power (W)", t0, t1, y0, y1, &[&pwr], params, opts)?;

    Ok(())
}
