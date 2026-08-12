//! PNG export of the rendered charts and opening them in the OS image viewer.
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process;

use plotters::prelude::*;

use crate::model::*;
use crate::render::{render_view, view_size, RenderParams};

/// Renders one view to `out_dir/<XX>_<view>.png` and returns its path.
pub fn render_view_to_file(
    view: View,
    sess: &Session,
    st: &Stats,
    params: &RenderParams,
    out_dir: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(out_dir)?;
    let size = view_size(view, st);
    let path = PathBuf::from(out_dir).join(view.file_name());
    {
        // Inner scope so the backend (which borrows `path`) is dropped before
        // we return the path.
        let root = BitMapBackend::new(&path, size).into_drawing_area();
        root.fill(&WHITE)?;
        render_view(view, sess, st, params, &root)?;
        root.present()?;
    }
    Ok(path)
}

/// Renders all six views to PNG files.
pub fn export_all(
    sess: &Session,
    st: &Stats,
    params: &RenderParams,
    out_dir: &str,
) -> Result<(), Box<dyn Error>> {
    for view in View::ALL {
        render_view_to_file(view, sess, st, params, out_dir)?;
    }
    Ok(())
}

fn open_image(path: &std::path::Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = process::Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(path)
            .spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = process::Command::new("xdg-open").arg(path).spawn();
    }
}

/// Opens already-rendered views in the OS viewer. `None` opens all.
pub fn show_view(view: Option<View>, out_dir: &str) {
    match view {
        Some(v) => open_image(&PathBuf::from(out_dir).join(v.file_name())),
        None => {
            for v in View::ALL {
                open_image(&PathBuf::from(out_dir).join(v.file_name()));
            }
        }
    }
}
