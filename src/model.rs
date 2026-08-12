//! Core data model shared by all modules.
use plotters::prelude::RGBColor;

/// Frame times above this value (ms) are treated as data corruption and ignored
/// for frame-time analysis. Real game frames (even catastrophic stalls) never
/// reach 10 seconds; garbage values such as `1.8e15` from corrupted CSV cells
/// do, and would otherwise blow up statistics and crash plotters.
pub const MAX_FRAME_MS: f32 = 10_000.0;

/// Threshold above which a frame is counted as a stutter (ms).
pub const STUTTER_MS: f32 = 50.0;

/// Number of buckets used when downsampling a time series for display.
pub const BUCKET_NUM: usize = 1500;

pub const IMG_W: u32 = 1700;
pub const IMG_H: u32 = 900;
/// Height used for the (taller) frame-time chart.
pub const IMG_H_TALL: u32 = 1050;

// TODO: Auto MAX_GPU
pub const MAX_GPU: usize = 2;

/// Maximum number of CPU cores we track (FrameView logs `CPUCoreUtil%[0..63]`).
pub const MAX_CORES: usize = 64;

pub const OUT_DIR: &str = "output";

/* Categorical palette (blue / orange / green / red). */
pub const C_SERIES_A: RGBColor = RGBColor(78, 121, 167);
pub const C_SERIES_B: RGBColor = RGBColor(242, 142, 43);
pub const C_AVG: RGBColor = RGBColor(89, 161, 79);
pub const C_STUTTER: RGBColor = RGBColor(225, 87, 89);
/* Neutral axis / grid tones. */
pub const C_GRID_LIGHT: RGBColor = RGBColor(231, 231, 236);
pub const C_GRID_BOLD: RGBColor = RGBColor(205, 205, 214);
pub const C_AXIS_TEXT: RGBColor = RGBColor(110, 110, 122);
pub const C_CAPTION: RGBColor = RGBColor(45, 48, 60);

/// One row of the FrameView capture log.
pub struct FrameSample {
    pub time_s: f32,
    pub frame_ms: f32,
    pub present_api_ms: f32,
    pub present_latency_ms: f32,
    pub until_displayed_ms: f32,
    pub render_queue: f32,
    #[allow(dead_code)] // kept as part of the parsed CSV model
    pub pc_latency_ms: f32,
    pub gpu_util: [f32; MAX_GPU],
    pub gpu_clk: [f32; MAX_GPU],
    pub gpu_mem_clk: [f32; MAX_GPU],
    pub gpu_temp: [f32; MAX_GPU],
    pub cpu_util: f32,
    pub cpu_clk: f32,
    pub cpu_temp: f32,
    pub cpu_power: f32,
    pub cpu_core_util: [f32; MAX_CORES],
}

/// Whole capture session.
pub struct Session {
    pub app: String,
    pub gpu: String,
    pub cpu: String,
    pub resolution: String,
    pub runtime: String,
    pub frames: Vec<FrameSample>,
}

/// Computed statistics.
#[derive(Default)]
pub struct Stats {
    pub frame_num: usize,
    pub duration_s: f32,
    pub avg_ms: f32,
    pub median_ms: f32,
    pub p99_ms: f32,
    pub avg_fps: f32,
    pub low1_fps: f32,
    pub low01_fps: f32,
    pub stutter_num: usize,
    pub stutter_total_ms: f32,
    pub worst_ms: f32,
    pub stutter_time: Vec<f32>,
    pub stutter_ms: Vec<f32>,
    pub stutter_idx: Vec<usize>,
}

/// The six charts produced from a capture.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    FrameTime,
    Report,
    Histogram,
    Gpu,
    Cpu,
    Latency,
}

impl View {
    pub const ALL: [View; 6] = [
        View::FrameTime,
        View::Report,
        View::Histogram,
        View::Gpu,
        View::Cpu,
        View::Latency,
    ];

    pub fn label(self) -> &'static str {
        match self {
            View::FrameTime => "Frame Time Timeline",
            View::Report => "Performance Report",
            View::Histogram => "Frame Time Histogram",
            View::Gpu => "GPU Monitor",
            View::Cpu => "CPU Monitor",
            View::Latency => "Latency Analysis",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            View::FrameTime => "frame_time",
            View::Report => "report",
            View::Histogram => "histogram",
            View::Gpu => "gpu",
            View::Cpu => "cpu",
            View::Latency => "latency",
        }
    }

    /// 1-based view number (matches the historical CLI numbering).
    pub fn number(self) -> usize {
        match self {
            View::FrameTime => 1,
            View::Report => 2,
            View::Histogram => 3,
            View::Gpu => 4,
            View::Cpu => 5,
            View::Latency => 6,
        }
    }

    /// Output file name, e.g. `01_frame_time.png`.
    pub fn file_name(self) -> String {
        format!("{:02}_{}.png", self.number(), self.slug())
    }
}

impl From<u32> for View {
    fn from(v: u32) -> View {
        match v {
            2 => View::Report,
            3 => View::Histogram,
            4 => View::Gpu,
            5 => View::Cpu,
            6 => View::Latency,
            _ => View::FrameTime,
        }
    }
}
