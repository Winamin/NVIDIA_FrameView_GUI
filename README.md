# NVIDIA FrameView GUI

一个基于 **Rust + plotters + egui/eframe** 的 **NVIDIA FrameView** 采集日志分析工具：
解析 FrameView 导出的 CSV 日志，渲染 6 张图表，并提供一个带实时调参、动画与缩放的交互式 GUI。

An **NVIDIA FrameView** capture-log analyzer built with **Rust + plotters + egui/eframe**:
it parses FrameView CSV logs, renders 6 charts, and ships an interactive GUI with
live parameters, animation, and zoom.

---

## Features / 功能

- **6 张图表**（6 charts）：帧时间线 / 性能报告 / 帧时间直方图 / GPU 监控 / CPU 监控 / 延迟分析
- **数据驱动的坐标轴**：y 轴按数据 min/max 自动缩放；直方图 x 轴聚焦真实帧时间分布
- **每张图标注最低点 / 最高点具体数值**（min/max 标注，可开关）
- **CPU 每核占用率**：每个核心一条不同颜色的曲线（`CPUCoreUtil%[N]`）
- **交互式 GUI**（eframe/egui）：
  - 一键切换 6 张图
  - 参数滑杆实时重绘：降采样桶数、直方图范围(%)、对数计数轴、y 轴留白、线宽、min/max 标注
  - 时间扫描动画（图表随时间"画出来"，直方图柱子生长）
  - 缩放：鼠标滚轮缩放、拖拽平移、`− / 1:1 / +` 按钮；放大时按更高分辨率重新渲染，文字清晰
  - 一键保存当前图为 PNG、加载另一个 CSV
- **数据清洗**：自动剔除损坏的帧时间（>10s 视为垃圾值），避免统计失真与渲染崩溃
- **命令行导出**：无窗口环境也能把 6 张图导出为 PNG
- **美化**：现代配色、浅灰网格、带色块的图例

---

## Build & Run / 构建与运行

要求：**Rust 2024 edition**（在 Rust 1.92 上测试通过）。

```bash
cargo build --release
```

运行：

```bash
# 默认：渲染 PNG 后打开交互式 GUI（不传 CSV 参数会弹出文件选择框）
./target/release/NVIDIA_FrameView_GUI.exe capture.csv

# 只导出 PNG（无头模式）
./target/release/NVIDIA_FrameView_GUI.exe capture.csv --export-only

# 导出时直方图使用对数 y 轴
./target/release/NVIDIA_FrameView_GUI.exe capture.csv --export-only --hist-log-y

# 老式终端菜单：渲染 PNG 并用系统看图器打开
./target/release/NVIDIA_FrameView_GUI.exe capture.csv --no-gui

# 指定打开第 3 张图（直方图）
./target/release/NVIDIA_FrameView_GUI.exe capture.csv --view 3
```

PNG 输出到 `output/` 目录（`01_frame_time.png` … `06_latency.png`）。

## CLI Reference / 命令行参考

| 参数 | 说明 |
| --- | --- |
| `<capture.csv>` | FrameView 导出的 CSV 日志；缺省则弹出文件选择框 |
| `--export-only` | 只渲染 PNG，不开任何窗口 |
| `--no-gui` | 渲染 PNG 后用系统看图器打开（终端菜单模式） |
| `--hist-log-y` | 直方图使用对数计数轴（配合 `--export-only`） |
| `--view N` / `-v N` / `--view=N` | 只打开第 N 张图（N=1..6） |

## GUI Usage / GUI 使用

左侧控制面板：

- **Views**：单选切换 6 张图
- **Parameters**（实时重绘）：
  - `downsample buckets` — 折线降采样桶数
  - `histogram range (%)` — 直方图 x 轴右端取的帧时间百分位（95–100，默认 99.5）
  - `Histogram log-scale count axis` — 直方图对数计数轴（让卡顿尾部可见）
  - `y padding` — y 轴留白比例
  - `line width (px)` — 序列线宽
  - `Show min / max labels` — 显示/隐藏最低最高标注
- **Animation**：`▶ Play sweep` 播放时间扫描动画；`sweep speed` 速度；`progress` 手动拖动进度
- **Zoom**：`−` / `1:1 fit` / `+` 按钮；图表上滚轮缩放、按住拖拽平移
- **Export**：`Save current chart as PNG` 保存当前图；`Load another CSV…` 加载新日志

---

## Charts / 图表说明

| # | 图 | 内容 |
| --- | --- | --- |
| 1 | **Frame Time Timeline** | 每桶平均/最大帧时间、全局平均、卡顿阈值线（50ms）、全局 min/max 标注、对数 y 轴、顶部统计条（帧数/时长/平均FPS/中位/p99/1%Low/0.1%Low/卡顿数） |
| 2 | **Performance Report** | 纯文本报告：平均/中位/p99 帧时间、平均 FPS、1%/0.1% Low、卡顿事件列表 |
| 3 | **Frame Time Histogram** | x 轴按数据分布自适应（`[p0.5, pX]`），峰值与平均线标注，超过范围的帧进入右侧红色溢出柱；可选对数 y 轴 |
| 4 | **GPU Monitor** | GPU0/GPU1 利用率 / 温度 / 核心频率 / 显存频率，y 轴按数据缩放 |
| 5 | **CPU Monitor** | 顶部整行**每核占用率**（每核一色）+ 聚合利用率 / 频率 / 温度 / 功耗 |
| 6 | **Latency Analysis** | 呈现延迟 / 直至显示 / Present API 内耗时 / 渲染队列深度 |

所有折线图都标注各序列的最低点与最高点数值。

## Metrics / 使用的指标列

按列名匹配（与列顺序无关）：

- 帧时间：`TimeInSeconds`, `MsBetweenPresents`
- 延迟：`MsInPresentAPI`, `MsRenderPresentLatency`, `MsUntilDisplayed`, `Render Queue Depth`
- GPU：`GPU0/1Util(%)`, `GPU0/1Clk(MHz)`, `GPU0/1MemClk(MHz)`, `GPU0/1Temp(C)`
- CPU：`CPUUtil(%)`, `CPUClk(MHz)`, `CPU Package Temp(C)`, `CPU Package Power(W)`, `CPUCoreUtil%[N]`（0–63）

**数据清洗**：`MsBetweenPresents` 中非有限值或超过 `MAX_FRAME_MS`（10 秒）的帧时间视为损坏数据置为 `NaN`，不计入统计与绘图，避免（例如 1.8e15ms 的垃圾值）把平均帧时间拉高或触发渲染溢出崩溃。

---

## Project Layout / 代码结构

```
src/
  main.rs              入口：CLI 解析、加载→分析→导出→启动 GUI
  model.rs             数据结构与常量（FrameSample, Session, Stats, View…）
  csv.rs               CSV 加载 + 帧时间清洗
  stats.rs             统计分析（平均/中位/p99/1%Low/卡顿）
  render/
    mod.rs             共享渲染（Series、downsample、draw_panel、min/max 标注、render_view 分发）
    frame_time.rs      帧时间线
    report.rs          性能报告
    histogram.rs       帧时间直方图
    gpu.rs             GPU 面板
    cpu.rs             CPU 面板（含每核）
    latency.rs         延迟面板
  export.rs            PNG 导出 + open_image/show_view
  gui.rs               eframe/egui 交互式查看器
```

## Tech Stack / 技术栈

[Rust](https://www.rust-lang.org/) · [plotters](https://docs.rs/plotters)（图表渲染）· [egui/eframe](https://github.com/emilk/egui)（GUI）· [csv](https://docs.rs/csv) · [rfd](https://docs.rs/rfd)（文件对话框）

FrameView 是 NVIDIA 的免费性能采集工具，详见 <https://www.nvidia.com/en-us/geforce/technologies/frameview/>。

---

## License / 许可证
MIT
