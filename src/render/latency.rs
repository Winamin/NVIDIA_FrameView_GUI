//! Latency analysis: present latency / until-displayed / present-API / queue depth.
use std::error::Error;

use crate::model::*;
use super::{draw_panel, make_series, padded_range, DArea, PanelOptions, RenderParams};

pub fn render_latency_area(
    sess: &Session,
    _st: &Stats,
    params: &RenderParams,
    area: &DArea<'_>,
) -> Result<(), Box<dyn Error>> {
    let t0 = sess.frames[0].time_s;
    let t1 = sess.frames[sess.frames.len() - 1].time_s;

    let pres = make_series(sess, |f: &FrameSample| f.present_latency_ms, C_SERIES_A, "PresentLat");
    let disp = make_series(sess, |f: &FrameSample| f.until_displayed_ms, C_SERIES_B, "UntilDisplayed");
    let api = make_series(sess, |f: &FrameSample| f.present_api_ms, C_SERIES_A, "InPresentAPI");
    let q = make_series(sess, |f: &FrameSample| f.render_queue, C_SERIES_B, "QueueDepth");

    let areas = area.split_evenly((2, 2));
    let opts = PanelOptions::default();

    let (y0, y1) = padded_range(&[&pres], params.y_pad);
    draw_panel(&areas[0], "Render Present Latency (ms)", t0, t1, y0, y1, &[&pres], params, opts)?;
    let (y0, y1) = padded_range(&[&disp], params.y_pad);
    draw_panel(&areas[1], "Until Displayed (ms)", t0, t1, y0, y1, &[&disp], params, opts)?;
    let (y0, y1) = padded_range(&[&api], params.y_pad);
    draw_panel(&areas[2], "In Present API (ms)", t0, t1, y0, y1, &[&api], params, opts)?;
    let (y0, y1) = padded_range(&[&q], params.y_pad);
    draw_panel(&areas[3], "Render Queue Depth", t0, t1, y0, y1, &[&q], params, opts)?;

    Ok(())
}
