use std::path::Path;

use blacklive::parse_midi_file;
use eframe::egui;

use super::App;

impl App {
    pub(crate) fn choose_midi(&mut self, ctx: egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("MIDI", &["mid", "midi", "MID"])
            .pick_file()
        {
            self.load_midi(path, ctx);
        }
    }

    pub(crate) fn load_midi(&mut self, path: std::path::PathBuf, ctx: egui::Context) {
        self.midi_path = path.display().to_string();
        self.output_path = path.with_extension("als").display().to_string();
        self.midi_stats = None;
        self.last_result = None;
        self.bpm_text.clear();
        self.bpm_edit = None;
        self.selected_track = Some(0);
        self.armed_tracks.clear();
        self.muted_tracks.clear();
        self.solo_tracks.clear();
        self.log(format!("选择 MIDI: {}", path.display()));
        self.set_status("正在解析 MIDI…");
        self.start_parse(ctx);
    }

    pub(crate) fn choose_template(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Ableton Live Set", &["als"])
            .pick_file()
        {
            self.template_path = path.display().to_string();
            self.template_report = None;
            self.selected_clip = None;
            self.last_result = None;
            self.log(format!("选择模板: {}", path.display()));
            self.analyze_template();
        }
    }

    pub(crate) fn template_is_builtin(&self) -> bool {
        self.template_path.trim().is_empty()
    }

    pub(crate) fn clear_template(&mut self) {
        self.template_path.clear();
        self.template_report = None;
        self.selected_clip = None;
        self.last_result = None;
        self.log("使用内置 Ac Piano Upright 模板");
        self.set_status("已切回内置模板。");
    }

    pub(crate) fn choose_output(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Ableton Live Set", &["als"])
            .set_file_name("output.als");
        if !self.output_path.trim().is_empty()
            && let Some(parent) = Path::new(self.output_path.trim()).parent()
            && !parent.as_os_str().is_empty()
        {
            dialog = dialog.set_directory(parent);
        }
        if let Some(path) = dialog.save_file() {
            self.output_path = path.display().to_string();
        }
    }

    pub(crate) fn analyze_template(&mut self) {
        let path = self.template_path.trim();
        let report = if path.is_empty() {
            blacklive::inspect_default_template()
        } else {
            blacklive::inspect_template(Path::new(path))
        };
        match report {
            Ok(report) => {
                self.log(format!(
                    "{}: Live {}，{} 条 MIDI 轨道，{} 个 Clip，BPM {}",
                    if path.is_empty() {
                        "内置模板"
                    } else {
                        "模板"
                    },
                    report.minor_version,
                    report.tracks.len(),
                    report.clips.len(),
                    report
                        .bpm
                        .map(blacklive::format_num)
                        .unwrap_or_else(|| "?".into())
                ));
                self.selected_clip = report.recommended_clip;
                self.template_report = Some(report);
                self.set_status("模板分析完成。");
            }
            Err(err) => {
                self.template_report = None;
                self.selected_clip = None;
                self.set_error(format!("{err:#}"));
            }
        }
    }

    pub(crate) fn analyze_midi(&mut self) {
        let path = self.midi_path.trim();
        if path.is_empty() {
            self.set_error("请先选择 MIDI");
            return;
        }
        match parse_midi_file(Path::new(path)) {
            Ok(parsed) => {
                let stats = parsed.stats;
                self.log(format!(
                    "MIDI: {} 音符，PPQ {}，初始 BPM {}，最大复音 {}",
                    stats.note_count,
                    stats.ppq,
                    stats
                        .initial_bpm
                        .map(blacklive::format_num)
                        .unwrap_or_else(|| "?".into()),
                    stats.max_polyphony
                ));
                self.midi_stats = Some(stats);
                self.set_status("MIDI 解析完成。");
            }
            Err(err) => {
                self.midi_stats = None;
                self.set_error(format!("{err:#}"));
            }
        }
    }

    pub(crate) fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|input| input.raw.dropped_files.clone());
        for file in dropped {
            let path = file.path().to_path_buf();
            if !path.is_file() {
                continue;
            }
            match path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .as_deref()
            {
                Some("mid") | Some("midi") => {
                    self.log(format!("拖入 MIDI: {}", path.display()));
                    self.load_midi(path, ctx.clone());
                }
                Some("als") => {
                    self.template_path = path.display().to_string();
                    self.template_report = None;
                    self.selected_clip = None;
                    self.last_result = None;
                    self.log(format!("拖入模板: {}", path.display()));
                    self.analyze_template();
                }
                _ => {}
            }
        }
    }

    pub(crate) fn dump_xml(&mut self) {
        let builtin = self.template_is_builtin();
        let template = self.template_path.trim();
        let default_name = if builtin {
            "default-template.xml".to_string()
        } else {
            Path::new(template)
                .with_extension("xml")
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "template.xml".into())
        };
        let Some(output) = rfd::FileDialog::new()
            .add_filter("XML", &["xml"])
            .set_file_name(default_name)
            .save_file()
        else {
            return;
        };
        let result = if builtin {
            blacklive::dump_default_template_xml(&output)
        } else {
            blacklive::als::dump_template_xml(Path::new(template), &output)
        };
        match result {
            Ok(()) => {
                self.log(format!("导出模板 XML: {}", output.display()));
                self.set_status(format!("已导出 XML：{}", output.display()));
            }
            Err(err) => self.set_error(format!("{err:#}")),
        }
    }
}
