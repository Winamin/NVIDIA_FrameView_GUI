//! Statistical analysis of a capture session.
use crate::model::{Session, Stats, STUTTER_MS};

fn percentile(sorted: &[f32], p: f32) -> f32 {
    let idx = ((sorted.len() - 1) as f32 * p).round() as usize;
    sorted[idx]
}

fn low_fps(sorted: &[f32], p: f32) -> f32 {
    let n = sorted.len();
    if n == 0 { return 0.0; }
    let k = ((n as f32 * p).ceil() as usize).max(1);
    let mut sum = 0.0f32;
    for i in (n - k)..n { sum += sorted[i]; }
    let avg = sum / k as f32;
    if avg > 0.0 {
        1000.0 / avg
    } else {
        0.0
    }
}

/// Computes frame-time statistics for the whole session. Frame rows whose
/// `frame_ms` is `NaN` (already filtered at load time) are ignored here.
pub fn analyze(sess: &Session, st: &mut Stats) {
    let n = sess.frames.len();
    let mut times: Vec<f32> = Vec::with_capacity(n);

    for f in &sess.frames {
        if f.frame_ms > 0.0 {
            times.push(f.frame_ms);
        }
    }
    st.frame_num = times.len();
    if st.frame_num == 0 {
        return;
    }

    if n > 1 {
        st.duration_s = sess.frames[n - 1].time_s - sess.frames[0].time_s;
        if st.duration_s > 0.0 {
            st.avg_fps = st.frame_num as f32 / st.duration_s;
        }
    }

    let mut sum: f32 = 0.0;
    for &v in &times {
        sum += v;
    }
    st.avg_ms = sum / st.frame_num as f32;

    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    st.median_ms = percentile(&times, 0.5);
    st.p99_ms = percentile(&times, 0.99);
    st.low1_fps = low_fps(&times, 0.01);
    st.low01_fps = low_fps(&times, 0.001);

    for i in 0..n {
        let v = sess.frames[i].frame_ms;
        if v > STUTTER_MS {
            st.stutter_idx.push(i);
            st.stutter_time.push(sess.frames[i].time_s);
            st.stutter_ms.push(v);
            st.stutter_total_ms += v;
            if v > st.worst_ms {
                st.worst_ms = v;
            }
        }
    }
    st.stutter_num = st.stutter_idx.len();
}
