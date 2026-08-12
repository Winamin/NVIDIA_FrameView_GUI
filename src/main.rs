mod csv;
mod export;
mod gui;
mod model;
mod render;
mod stats;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

use model::{Session, Stats, View, OUT_DIR};
use render::RenderParams;
use stats::analyze;

fn build_session(path: &str) -> Result<(Session, Stats), Box<dyn std::error::Error>> {
    let mut sess = Session {
        app: String::new(),
        gpu: String::new(),
        cpu: String::new(),
        resolution: String::new(),
        runtime: String::new(),
        frames: Vec::new(),
    };
    csv::load_csv(path, &mut sess)?;
    let mut st = Stats::default();
    analyze(&sess, &mut st);
    Ok((sess, st))
}

/// Terminal viewer menu used in `--no-gui` mode.
fn run_menu(out_dir: &str) {
    loop {
        println!();
        for v in View::ALL {
            println!("  [{}] {}", v.number(), v.label());
        }
        println!("  [0] Open all    [q] Quit");
        print!("  Select: ");
        io::stdout().flush().ok();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() || input.trim().is_empty() {
            break;
        }
        let t = input.trim();
        if t.eq_ignore_ascii_case("q") {
            break;
        }
        match t.parse::<u32>() {
            Ok(0) => export::show_view(None, out_dir),
            Ok(v) if (1..=6).contains(&v) => export::show_view(Some(View::from(v)), out_dir),
            _ => eprintln!("Enter a number or q"),
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut no_gui = false;
    let mut export_only = false;
    let mut hist_log_y = false;
    let mut direct_view: Option<View> = None;
    let mut path: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--no-gui" => no_gui = true,
            "--hist-log-y" => hist_log_y = true,
            "--export-only" => {
                export_only = true;
                no_gui = true;
            }
            "--view" | "-v" => {
                if let Some(v) = args.get(i + 1) {
                    direct_view = Some(View::from(v.parse().unwrap_or(1)));
                }
                i += 1;
            }
            _ => {
                if let Some(rest) = a.strip_prefix("--view=") {
                    direct_view = Some(View::from(rest.parse().unwrap_or(1)));
                } else if a.starts_with('-') && a.len() > 1 {
                    eprintln!("Unknown option: {a}");
                } else if path.is_none() {
                    path = Some(a.clone());
                }
            }
        }
        i += 1;
    }

    let path = path.unwrap_or_else(|| {
        println!("Please select a CSV file...");
        match rfd::FileDialog::new()
            .add_filter("CSV files", &["csv"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            Some(file) => file.to_string_lossy().to_string(),
            None => {
                eprintln!("No file selected. Exiting.");
                process::exit(1);
            }
        }
    });

    let (sess, st) = match build_session(&path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to load {path}: {e}");
            process::exit(1);
        }
    };

    fs::create_dir_all(OUT_DIR).ok();
    let mut params = RenderParams::default();
    if hist_log_y {
        params.hist_log_y = true;
    }
    match export::export_all(&sess, &st, &params, OUT_DIR) {
        Ok(()) => println!("Charts saved to {OUT_DIR}/"),
        Err(e) => eprintln!("Export failed: {e}"),
    }

    if export_only {
        return;
    }

    if no_gui {
        match direct_view {
            Some(v) => export::show_view(Some(v), OUT_DIR),
            None => {
                export::show_view(Some(View::FrameTime), OUT_DIR);
                run_menu(OUT_DIR);
            }
        }
        return;
    }

    /* Default: interactive GUI. Fall back to opening the saved PNGs if the
       window can't start (e.g. headless session). */
    if let Err(e) = gui::run(sess, st) {
        eprintln!("GUI failed to start: {e}\nOpening saved charts instead.");
        match direct_view {
            Some(v) => export::show_view(Some(v), OUT_DIR),
            None => export::show_view(Some(View::FrameTime), OUT_DIR),
        }
    }
}
