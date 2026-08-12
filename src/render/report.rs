//! Plain-text performance report chart.
use std::error::Error;

use crate::model::*;
use super::{draw_text, DArea};

/// Natural canvas size for the report (grows with the number of stutter events).
pub fn report_size(st: &Stats) -> (u32, u32) {
    let shown = st.stutter_num.min(40);
    (1200, (500 + shown * 20) as u32)
}

pub fn render_report_area(sess: &Session, st: &Stats, area: &DArea<'_>) -> Result<(), Box<dyn Error>> {
    let shown = st.stutter_num.min(40);

    let mut y = 30i32;
    let mut line = |s: &str, size: u32| -> Result<(), Box<dyn Error>> {
        draw_text(area, s, 30, y, size)?;
        y += size as i32 + 14;
        Ok(())
    };

    line("FrameView Performance Report", 36)?;
    line(&format!("Application: {}", sess.app), 22)?;
    line(&format!("GPU: {}", sess.gpu), 22)?;
    line(&format!("CPU: {}", sess.cpu), 22)?;
    line(&format!("Resolution: {}   API: {}", sess.resolution, sess.runtime), 22)?;
    line(&format!("Frames: {}   Duration: {:.1} s", st.frame_num, st.duration_s), 22)?;
    line("---------------------------------------------", 22)?;
    line(&format!("Average Frame Time : {:.3} ms", st.avg_ms), 24)?;
    line(&format!("Average FPS        : {:.1}", st.avg_fps), 24)?;
    line(&format!("Median (p50)       : {:.3} ms", st.median_ms), 24)?;
    line(&format!("p99 Frame Time     : {:.3} ms", st.p99_ms), 24)?;
    line(&format!("1% Low FPS         : {:.1}", st.low1_fps), 24)?;
    line(&format!("0.1% Low FPS       : {:.1}", st.low01_fps), 24)?;
    line(
        &format!(
            "Stutters (> {:.0} ms): {}   Total: {:.2} s   Worst: {:.2} ms",
            STUTTER_MS,
            st.stutter_num,
            st.stutter_total_ms / 1000.0,
            st.worst_ms
        ),
        24,
    )?;
    line("---------------------------------------------", 22)?;
    line("Stutter events (frame #, time, frame time):", 24)?;
    for i in 0..shown {
        line(
            &format!(
                "  #{:<7}  t = {:>10.3} s     {:>9.3} ms",
                st.stutter_idx[i], st.stutter_time[i], st.stutter_ms[i]
            ),
            20,
        )?;
    }
    if st.stutter_num > shown {
        line(&format!("  ... and {} more", st.stutter_num - shown), 20)?;
    }

    Ok(())
}
