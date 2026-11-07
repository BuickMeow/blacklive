use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

#[derive(Default)]
pub(crate) struct FileActions {
    pub(crate) choose_midi: bool,
    pub(crate) choose_template: bool,
    pub(crate) clear_template: bool,
    pub(crate) choose_output: bool,
    pub(crate) analyze_template: bool,
    pub(crate) analyze_midi: bool,
    pub(crate) dump_xml: bool,
}

#[derive(Default)]
pub(crate) struct ActionFlags {
    pub(crate) convert: bool,
    pub(crate) open_result: Option<PathBuf>,
    pub(crate) open_folder: Option<PathBuf>,
}

pub(crate) fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

pub(crate) fn open_in_default(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = Command::new("xdg-open");

    command
        .arg(path)
        .spawn()
        .with_context(|| format!("无法打开 {}", path.display()))?;
    Ok(())
}

pub(crate) fn reveal_in_folder(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "macos")]
    command.arg("-R").arg(path);

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = Command::new("explorer");
        c.arg(format!("/select,{}", path.display()));
        c
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut c = Command::new("xdg-open");
        c.arg(path.parent().unwrap_or_else(|| Path::new(".")));
        c
    };

    command
        .spawn()
        .with_context(|| format!("无法打开文件夹: {}", path.display()))?;
    Ok(())
}
