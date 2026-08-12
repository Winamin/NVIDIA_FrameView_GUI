//! Frame-time timeline chart (the "hero" chart, with stats banner on top).
use std::error::Error;

use plotters::prelude::*;

use crate::model::*;
use super::{clamp_pts, draw_label_at, draw_text, downsample, sweep_cut, BucketMode, DArea, RenderParams};

pub fn render_frame_time_area(
    sess: &Session,
    st: &Stats,
    params: &RenderParams,
    area: &DArea<'_>,
) -> Result<(), Box<dyn Error>> {
    let mut pts: Vec<(f32, f32)> = Vec::with_capacity(sess.frames.len());
    for f in &sess.frames {
        if f.frame_ms > 0.0 {
            pts.push((f.time_s, f.frame_ms));
        }
    }
    if pts.is_empty() {
        return Ok(());
    }
    let t0 = pts[0].0;
    let t1 = pts[pts.len() - 1].0;
    let mut max_ms = 0.0f32;
    for p in &pts {
        if p.1 > max_ms {
            max_ms = p.1;
        }
    }
    let y_max = (max_ms.max(STUTTER_MS * 2.0) * (1.0 + params.y_pad)).max(100.0);

    let cut = sweep_cut(params, t0, t1);

    /* Stats banner (all analysis lives on the image, not the terminal). */
    draw_text(area, &format!("App: {}   GPU: {}   CPU: {}", sess.app, sess.gpu, sess.cpu), 20, 60, 20)?;
    draw_text(
        area,
        &format!(
            "Frames: {}   Duration: {:.1} s   Avg FPS: {:.1}",
            st.frame_num, st.duration_s, st.avg_fps
        ),
        20,
        95,
        20,
    )?;
    draw_text(
        area,
        &format!(
            "Avg Frame: {:.2} ms   Median: {:.2} ms   p99: {:.2} ms",
            st.avg_ms, st.median_ms, st.p99_ms
        ),
        20,
        130,
        20,
    )?;
    draw_text(
        area,
        &format!("1% Low: {:.1} FPS   0.1% Low: {:.1} FPS", st.low1_fps, st.low01_fps),
        20,
        165,
        20,
    )?;
    draw_text(
        area,
        &format!(
            "Stutters (> {:.0} ms): {}   Total: {:.2} s   Worst: {:.2} ms",
            STUTTER_MS,
            st.stutter_num,
            st.stutter_total_ms / 1000.0,
            st.worst_ms
        ),
        20,
        200,
        20,
    )?;

    let mut chart = ChartBuilder::on(area)
        .caption("Frame Time Timeline", TextStyle::from(("sans-serif", 30)).color(&C_CAPTION))
        .margin_top(240)
        .margin_left(25)
        .margin_right(25)
        .margin_bottom(25)
        .x_label_area_size(60)
        .y_label_area_size(85)
        .build_cartesian_2d(t0..t1, (0.5..y_max).log_scale())?;

    chart
        .configure_mesh()
        .x_desc("Time (s)")
        .y_desc("Frame time (ms, log)")
        .axis_desc_style(TextStyle::from(("sans-serif", 20)).color(&C_AXIS_TEXT))
        .label_style(TextStyle::from(("sans-serif", 16)).color(&C_AXIS_TEXT))
        .light_line_style(C_GRID_LIGHT)
        .bold_line_style(C_GRID_BOLD)
        .draw()?;

    /* Data revealed up to the sweep cursor, if animating. */
    let visible: Vec<(f32, f32)> = match cut {
        Some(c) => pts.iter().copied().filter(|p| p.0 <= c).collect(),
        None => pts.clone(),
    };
    let avg_line = clamp_pts(&downsample(&visible, params.bucket_num, BucketMode::Avg), t0, t1, 0.5, y_max);
    let max_line = clamp_pts(&downsample(&visible, params.bucket_num, BucketMode::Max), t0, t1, 0.5, y_max);
    let stutters: Vec<(f32, f32)> = visible
        .iter()
        .filter(|p| p.1 > STUTTER_MS)
        .copied()
        .collect();

    chart
        .draw_series(LineSeries::new(avg_line, C_SERIES_A.stroke_width(params.line_width)))?
        .label("avg per bucket")
        .legend(|p: (i32, i32)| PathElement::new(vec![(p.0, p.1), (p.0 + 20, p.1)], C_SERIES_A.stroke_width(3)));
    chart
        .draw_series(LineSeries::new(max_line, C_SERIES_B.stroke_width(params.line_width.saturating_sub(1).max(1))))?
        .label("max per bucket")
        .legend(|p: (i32, i32)| PathElement::new(vec![(p.0, p.1), (p.0 + 20, p.1)], C_SERIES_B.stroke_width(3)));

    if st.avg_ms > 0.0 {
        chart
            .draw_series(LineSeries::new(
                [(t0, st.avg_ms), (t1, st.avg_ms)],
                C_AVG.mix(0.85).stroke_width(3),
            ))?
            .label("global avg")
            .legend(|p: (i32, i32)| PathElement::new(vec![(p.0, p.1), (p.0 + 20, p.1)], C_AVG.stroke_width(3)));
        if params.show_extremes && cut.is_none() {
            draw_label_at(&mut chart, (t1, st.avg_ms), (-(t1 - t0) * 0.05, 0.0), format!("avg {:.2} ms", st.avg_ms), C_AVG)?;
        }
    }
    chart
        .draw_series(LineSeries::new(
            [(t0, STUTTER_MS), (t1, STUTTER_MS)],
            C_STUTTER.mix(0.55).stroke_width(2),
        ))?
        .label("stutter threshold")
        .legend(|p: (i32, i32)| PathElement::new(vec![(p.0, p.1), (p.0 + 20, p.1)], C_STUTTER.stroke_width(3)));

    chart.draw_series(PointSeries::of_element(
        stutters,
        5,
        C_STUTTER.filled(),
        &|coord, size, style| EmptyElement::at(coord) + Circle::new((0, 0), size, style),
    ))?;

    /* Sweep cursor line. */
    if let Some(c) = cut {
        if c > t0 && c < t1 {
            chart.draw_series(LineSeries::new(
                [(c, 0.5), (c, y_max)],
                RGBColor(90, 90, 90).mix(0.5).stroke_width(1),
            ))?;
        }
    }

    /* Global min/max frame-time annotations. */
    if params.show_extremes && cut.is_none() {
        let (min_pt, max_pt) = (pts.iter().copied().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap(),
                                pts.iter().copied().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap());
        chart.draw_series(PointSeries::of_element(
            vec![min_pt],
            5,
            C_AVG.filled(),
            &|c, s, st| EmptyElement::at(c) + Circle::new((0, 0), s, st),
        ))?;
        draw_label_at(&mut chart, min_pt, (-(t1 - t0) * 0.03, 0.0), format!("min {:.2} ms", min_pt.1), C_AVG)?;
        chart.draw_series(PointSeries::of_element(
            vec![max_pt],
            5,
            C_STUTTER.filled(),
            &|c, s, st| EmptyElement::at(c) + Circle::new((0, 0), s, st),
        ))?;
        draw_label_at(&mut chart, max_pt, ((t1 - t0) * 0.02, 0.0), format!("max {:.2} ms", max_pt.1), C_STUTTER)?;
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
