//! GPU monitor: utilization / temperature / clock / memory-clock panels.
//! Y axes are auto-scaled to the data range (with padding) rather than fixed.
use std::error::Error;

use crate::model::*;
use super::{draw_panel, make_series, padded_range, DArea, PanelOptions, RenderParams};

pub fn render_gpu_area(
    sess: &Session,
    _st: &Stats,
    params: &RenderParams,
    area: &DArea<'_>,
) -> Result<(), Box<dyn Error>> {
    let t0 = sess.frames[0].time_s;
    let t1 = sess.frames[sess.frames.len() - 1].time_s;

    let u0 = make_series(sess, |f: &FrameSample| f.gpu_util[0], C_SERIES_A, "GPU0");
    let u1 = make_series(sess, |f: &FrameSample| f.gpu_util[1], C_SERIES_B, "GPU1");
    let tm0 = make_series(sess, |f: &FrameSample| f.gpu_temp[0], C_SERIES_A, "GPU0");
    let tm1 = make_series(sess, |f: &FrameSample| f.gpu_temp[1], C_SERIES_B, "GPU1");
    let ck0 = make_series(sess, |f: &FrameSample| f.gpu_clk[0], C_SERIES_A, "GPU0");
    let ck1 = make_series(sess, |f: &FrameSample| f.gpu_clk[1], C_SERIES_B, "GPU1");
    let mk0 = make_series(sess, |f: &FrameSample| f.gpu_mem_clk[0], C_SERIES_A, "GPU0");
    let mk1 = make_series(sess, |f: &FrameSample| f.gpu_mem_clk[1], C_SERIES_B, "GPU1");

    let areas = area.split_evenly((2, 2));
    let opts = PanelOptions::default();

    let (y0, y1) = padded_range(&[&u0, &u1], params.y_pad);
    draw_panel(&areas[0], "GPU Utilization (%)", t0, t1, y0, y1, &[&u0, &u1], params, opts)?;
    let (y0, y1) = padded_range(&[&tm0, &tm1], params.y_pad);
    draw_panel(&areas[1], "GPU Temperature (C)", t0, t1, y0, y1, &[&tm0, &tm1], params, opts)?;
    let (y0, y1) = padded_range(&[&ck0, &ck1], params.y_pad);
    draw_panel(&areas[2], "GPU Clock (MHz)", t0, t1, y0, y1, &[&ck0, &ck1], params, opts)?;
    let (y0, y1) = padded_range(&[&mk0, &mk1], params.y_pad);
    draw_panel(&areas[3], "GPU Memory Clock (MHz)", t0, t1, y0, y1, &[&mk0, &mk1], params, opts)?;

    Ok(())
}
