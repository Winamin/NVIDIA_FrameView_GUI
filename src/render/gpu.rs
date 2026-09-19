//! GPU monitor: utilization / temperature / clock / memory-clock panels.
//! Y axes are auto-scaled to the data range (with padding) rather than fixed.
use std::error::Error;

use crate::model::*;
use super::{core_color, draw_panel, make_series, padded_range, time_range, DArea, PanelOptions, RenderParams};

pub fn render_gpu_area(
    sess: &Session,
    _st: &Stats,
    params: &RenderParams,
    area: &DArea<'_>,
) -> Result<(), Box<dyn Error>> {
    let (t0, t1) = time_range(sess);

    // GPU count comes from the CSV header; fall back to the actual row width
    // in case of a hand-built Session.
    let n = sess
        .gpu_count
        .max(sess.frames.first().map(|f| f.gpu_util.len()).unwrap_or(0));

    let mut utils: Vec<crate::render::Series> = Vec::with_capacity(n);
    let mut temps: Vec<crate::render::Series> = Vec::with_capacity(n);
    let mut clks: Vec<crate::render::Series> = Vec::with_capacity(n);
    let mut mems: Vec<crate::render::Series> = Vec::with_capacity(n);
    for i in 0..n {
        let name = format!("GPU{i}");
        let color = match i {
            0 => C_SERIES_A,
            1 => C_SERIES_B,
            _ => core_color(i, n),
        };
        let idx = i;
        utils.push(make_series(
            sess,
            move |f: &FrameSample| f.gpu_util.get(idx).copied().unwrap_or(f32::NAN),
            color,
            &name,
        ));
        let idx = i;
        temps.push(make_series(
            sess,
            move |f: &FrameSample| f.gpu_temp.get(idx).copied().unwrap_or(f32::NAN),
            color,
            &name,
        ));
        let idx = i;
        clks.push(make_series(
            sess,
            move |f: &FrameSample| f.gpu_clk.get(idx).copied().unwrap_or(f32::NAN),
            color,
            &name,
        ));
        let idx = i;
        mems.push(make_series(
            sess,
            move |f: &FrameSample| f.gpu_mem_clk.get(idx).copied().unwrap_or(f32::NAN),
            color,
            &name,
        ));
    }

    let areas = area.split_evenly((2, 2));
    let opts = PanelOptions::default();

    let u_refs: Vec<&crate::render::Series> = utils.iter().collect();
    let t_refs: Vec<&crate::render::Series> = temps.iter().collect();
    let c_refs: Vec<&crate::render::Series> = clks.iter().collect();
    let m_refs: Vec<&crate::render::Series> = mems.iter().collect();

    let (y0, y1) = padded_range(&u_refs, params.y_pad);
    draw_panel(&areas[0], "GPU Utilization (%)", t0, t1, y0, y1, &u_refs, params, opts)?;
    let (y0, y1) = padded_range(&t_refs, params.y_pad);
    draw_panel(&areas[1], "GPU Temperature (C)", t0, t1, y0, y1, &t_refs, params, opts)?;
    let (y0, y1) = padded_range(&c_refs, params.y_pad);
    draw_panel(&areas[2], "GPU Clock (MHz)", t0, t1, y0, y1, &c_refs, params, opts)?;
    let (y0, y1) = padded_range(&m_refs, params.y_pad);
    draw_panel(&areas[3], "GPU Memory Clock (MHz)", t0, t1, y0, y1, &m_refs, params, opts)?;

    Ok(())
}
