//! CSV loading for NVIDIA FrameView capture logs.
use std::collections::HashMap;
use std::error::Error;

use crate::model::{FrameSample, MAX_CORES, MAX_FRAME_MS, MAX_GPU, Session};

fn cell(rec: &csv::StringRecord, idx: usize) -> &str {
    match rec.get(idx) {
        Some(s) => s.trim(),
        None => "",
    }
}

fn cell_f32(rec: &csv::StringRecord, idx: usize) -> f32 {
    let s = cell(rec, idx);
    match s.parse::<f32>() {
        Ok(v) if v.is_finite() => v,
        _ => f32::NAN,
    }
}

pub fn load_csv(path: &str, sess: &mut Session) -> Result<(), Box<dyn Error>> {
    let mut rdr = csv::Reader::from_path(path)?;
    let headers = rdr.headers()?.clone();

    let mut col_map: HashMap<&str, usize> = HashMap::new();
    for (i, h) in headers.iter().enumerate() {
        col_map.insert(h, i);
    }
    let col = |name: &str| col_map.get(name).copied().unwrap_or(usize::MAX);

    let i_app = col("Application");
    let i_gpu = col("GPU");
    let i_cpu = col("CPU");
    let i_res = col("Resolution");
    let i_rt = col("Runtime");

    let i_time = col("TimeInSeconds");
    let i_frame = col("MsBetweenPresents");
    let i_present_api = col("MsInPresentAPI");
    let i_present_latency = col("MsRenderPresentLatency");
    let i_until_displayed = col("MsUntilDisplayed");
    let i_render_queue = col("Render Queue Depth");
    let i_pc_latency = col("MsPCLatency");

    let i_gpu0_util = col("GPU0Util(%)");
    let i_gpu0_clk = col("GPU0Clk(MHz)");
    let i_gpu0_mem = col("GPU0MemClk(MHz)");
    let i_gpu0_temp = col("GPU0Temp(C)");
    let i_gpu1_util = col("GPU1Util(%)");
    let i_gpu1_clk = col("GPU1Clk(MHz)");
    let i_gpu1_mem = col("GPU1MemClk(MHz)");
    let i_gpu1_temp = col("GPU1Temp(C)");

    let i_cpu_util = col("CPUUtil(%)");
    let i_cpu_clk = col("CPUClk(MHz)");
    let i_cpu_temp = col("CPU Package Temp(C)");
    let i_cpu_power = col("CPU Package Power(W)");

    // Per-core utilization columns are named `CPUCoreUtil%[ N]` — single-digit
    // indices are space-padded (`[ 0]`..`[ 9]`), double-digit are not. Resolve
    // each index once by scanning the header, tolerating the padding.
    let mut core_cols = [usize::MAX; MAX_CORES];
    for (name, &idx) in &col_map {
        if let Some(rest) = name.strip_prefix("CPUCoreUtil%[") {
            if let Some(end) = rest.find(']') {
                if let Ok(c) = rest[..end].trim().parse::<usize>() {
                    if c < MAX_CORES {
                        core_cols[c] = idx;
                    }
                }
            }
        }
    }

    let mut first = true;
    for rec in rdr.records() {
        let rec = rec?;
        if first {
            sess.app = cell(&rec, i_app).to_string();
            sess.gpu = cell(&rec, i_gpu).to_string();
            sess.cpu = cell(&rec, i_cpu).to_string();
            sess.resolution = cell(&rec, i_res).to_string();
            sess.runtime = cell(&rec, i_rt).to_string();
            first = false;
        }

        let mut f = FrameSample {
            time_s: cell_f32(&rec, i_time),
            frame_ms: 0.0,
            present_api_ms: cell_f32(&rec, i_present_api),
            present_latency_ms: cell_f32(&rec, i_present_latency),
            until_displayed_ms: cell_f32(&rec, i_until_displayed),
            render_queue: cell_f32(&rec, i_render_queue),
            pc_latency_ms: cell_f32(&rec, i_pc_latency),
            gpu_util: [f32::NAN; MAX_GPU],
            gpu_clk: [f32::NAN; MAX_GPU],
            gpu_mem_clk: [f32::NAN; MAX_GPU],
            gpu_temp: [f32::NAN; MAX_GPU],
            cpu_util: cell_f32(&rec, i_cpu_util),
            cpu_clk: cell_f32(&rec, i_cpu_clk),
            cpu_temp: cell_f32(&rec, i_cpu_temp),
            cpu_power: cell_f32(&rec, i_cpu_power),
            cpu_core_util: [f32::NAN; MAX_CORES],
        };

        // Per-core utilization. Columns the CPU doesn't have are absent from
        // the header and stay NaN.
        for c in 0..MAX_CORES {
            f.cpu_core_util[c] = cell_f32(&rec, core_cols[c]);
        }

        // Sanitize the frame time: corrupted values (e.g. `1.8e15`) are
        // physically impossible and would corrupt statistics and crash the
        // histogram renderer. The row is kept so GPU/CPU columns stay valid.
        f.frame_ms = cell_f32(&rec, i_frame);
        if !f.frame_ms.is_finite() || f.frame_ms > MAX_FRAME_MS {
            f.frame_ms = f32::NAN;
        }

        f.gpu_util[0] = cell_f32(&rec, i_gpu0_util);
        f.gpu_clk[0] = cell_f32(&rec, i_gpu0_clk);
        f.gpu_mem_clk[0] = cell_f32(&rec, i_gpu0_mem);
        f.gpu_temp[0] = cell_f32(&rec, i_gpu0_temp);
        f.gpu_util[1] = cell_f32(&rec, i_gpu1_util);
        f.gpu_clk[1] = cell_f32(&rec, i_gpu1_clk);
        f.gpu_mem_clk[1] = cell_f32(&rec, i_gpu1_mem);
        f.gpu_temp[1] = cell_f32(&rec, i_gpu1_temp);

        sess.frames.push(f);
    }

    if sess.frames.is_empty() {
        return Err("no data rows parsed".into());
    }
    Ok(())
}
