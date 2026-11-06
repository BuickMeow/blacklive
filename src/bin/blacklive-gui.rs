use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use blacklive::{
    ConversionResult, ConvertOptions, MidiStats, TemplateReport, convert_midi_to_als,
    parse_channel_filter, parse_midi_file,
};
use eframe::egui::{self, Color32, RichText};

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1020.0, 780.0])
            .with_min_inner_size([820.0, 600.0])
            .with_title("Blacklive — MIDI → Ableton .als"),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "Blacklive",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    ) {
        eprintln!("无法启动 GUI: {err}");
    }
}

struct App {
    midi_path: String,
    template_path: String,
    output_path: String,

    channels: String,
    include_drum_channel: bool,
    transpose: i32,
    bpm_text: String,
    min_note_beats: f64,
    tail_beats: f64,
    start_beats: f64,
    trim_same_pitch: bool,
    clip_name: String,
    track_name: String,

    template_report: Option<TemplateReport>,
    selected_clip: Option<usize>,
    midi_stats: Option<MidiStats>,
    last_result: Option<ConversionResult>,

    status: String,
    status_is_error: bool,
    log: Vec<String>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut app = Self {
            midi_path: String::new(),
            template_path: String::new(),
            output_path: String::new(),
            channels: String::new(),
            include_drum_channel: false,
            transpose: 0,
            bpm_text: String::new(),
            min_note_beats: 0.0,
            tail_beats: 0.0,
            start_beats: 0.0,
            trim_same_pitch: false,
            clip_name: String::new(),
            track_name: String::new(),
            template_report: None,
            selected_clip: None,
            midi_stats: None,
            last_result: None,
            status: "选择 MIDI 和模板 .als，然后点击「转换并生成 .als」。".to_string(),
            status_is_error: false,
            log: Vec::new(),
        };

        let args: Vec<String> = std::env::args().skip(1).collect();
        if let Some(path) = args.first() {
            app.midi_path = path.clone();
        }
        if let Some(path) = args.get(1) {
            app.template_path = path.clone();
        }
        app
    }

    fn set_status(&mut self, message: impl Into<String>) {
        self.status = message.into();
        self.status_is_error = false;
    }

    fn set_error(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.status = message.clone();
        self.status_is_error = true;
        self.log.push(format!("! {message}"));
    }

    fn log(&mut self, message: impl Into<String>) {
        self.log.push(message.into());
    }

    fn choose_midi(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("MIDI", &["mid", "midi", "MID"])
            .pick_file()
        {
            self.midi_path = path.display().to_string();
            self.output_path = path.with_extension("als").display().to_string();
            self.midi_stats = None;
            self.last_result = None;
            self.log(format!("选择 MIDI: {}", path.display()));
        }
    }

    fn choose_template(&mut self) {
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

    fn choose_output(&mut self) {
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

    fn analyze_template(&mut self) {
        let path = self.template_path.trim();
        if path.is_empty() {
            self.set_error("请先选择模板 .als");
            return;
        }
        match blacklive::inspect_template(Path::new(path)) {
            Ok(report) => {
                self.log(format!(
                    "模板: Live {}，{} 条 MIDI 轨道，{} 个 Clip，BPM {}",
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
                self.set_status("模板分析完成。请确认要替换的 Clip。");
            }
            Err(err) => {
                self.template_report = None;
                self.selected_clip = None;
                self.set_error(format!("{err:#}"));
            }
        }
    }

    fn analyze_midi(&mut self) {
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

    fn convert(&mut self) {
        let midi = PathBuf::from(self.midi_path.trim());
        let template = PathBuf::from(self.template_path.trim());
        if midi.as_os_str().is_empty() {
            self.set_error("请先选择 MIDI");
            return;
        }
        if template.as_os_str().is_empty() {
            self.set_error("请先选择模板 .als");
            return;
        }
        let output = if self.output_path.trim().is_empty() {
            midi.with_extension("als")
        } else {
            PathBuf::from(self.output_path.trim())
        };

        let bpm_override = match self.bpm_text.trim() {
            "" => None,
            value => match value.parse::<f64>() {
                Ok(bpm) if bpm.is_finite() && bpm > 0.0 => Some(bpm),
                _ => {
                    self.set_error(format!("BPM「{value}」不是正数"));
                    return;
                }
            },
        };
        let channels = match parse_channel_filter(&self.channels) {
            Ok(channels) => channels,
            Err(err) => {
                self.set_error(format!("{err:#}"));
                return;
            }
        };

        let options = ConvertOptions {
            template_clip_index: self.selected_clip,
            template_clip_name: None,
            channels,
            include_drum_channel: self.include_drum_channel,
            transpose: self.transpose,
            bpm_override,
            min_note_beats: self.min_note_beats,
            tail_beats: self.tail_beats,
            start_beats: self.start_beats,
            clip_name: non_empty(&self.clip_name),
            track_name: non_empty(&self.track_name),
            trim_same_pitch: self.trim_same_pitch,
        };

        self.set_status("正在转换…");
        match convert_midi_to_als(&midi, &template, &output, &options) {
            Ok(result) => {
                self.log(format!(
                    "✓ 输出 {}，写入 {} 音符，BPM {}，Clip {:.2} beats",
                    result.output_path.display(),
                    result.used_note_count,
                    blacklive::format_num(result.bpm),
                    result.clip_length_beats
                ));
                for warning in &result.warnings {
                    self.log(format!("⚠ {warning}"));
                }
                self.midi_stats = Some(result.midi_stats.clone());
                self.set_status(format!("转换完成：{}", result.output_path.display()));
                self.last_result = Some(result);
            }
            Err(err) => {
                self.last_result = None;
                self.set_error(format!("{err:#}"));
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
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
                    self.midi_path = path.display().to_string();
                    self.output_path = path.with_extension("als").display().to_string();
                    self.midi_stats = None;
                    self.last_result = None;
                    self.log(format!("拖入 MIDI: {}", path.display()));
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

    fn show_files(&mut self, ui: &mut egui::Ui) -> FileActions {
        let mut actions = FileActions::default();

        egui::Grid::new("files")
            .num_columns(3)
            .spacing([8.0, 8.0])
            .show(ui, |ui| {
                ui.label("MIDI");
                let response =
                    ui.add(egui::TextEdit::singleline(&mut self.midi_path).desired_width(620.0));
                if response.changed() {
                    self.midi_stats = None;
                    self.last_result = None;
                }
                if ui.button("选择…").clicked() {
                    actions.choose_midi = true;
                }
                ui.end_row();

                ui.label("模板 .als");
                let response = ui
                    .add(egui::TextEdit::singleline(&mut self.template_path).desired_width(620.0));
                if response.changed() {
                    self.template_report = None;
                    self.selected_clip = None;
                    self.last_result = None;
                }
                ui.horizontal(|ui| {
                    if ui.button("选择…").clicked() {
                        actions.choose_template = true;
                    }
                    if ui.button("分析模板").clicked() {
                        actions.analyze_template = true;
                    }
                });
                ui.end_row();

                ui.label("输出 .als");
                ui.add(egui::TextEdit::singleline(&mut self.output_path).desired_width(620.0));
                if ui.button("另存为…").clicked() {
                    actions.choose_output = true;
                }
                ui.end_row();
            });

        ui.horizontal(|ui| {
            if ui.button("解析 MIDI").clicked() {
                actions.analyze_midi = true;
            }
            if ui.button("导出模板 XML").clicked() {
                actions.dump_xml = true;
            }
        });
        actions
    }

    fn show_template(&mut self, ui: &mut egui::Ui) {
        let Some(report) = &self.template_report else {
            return;
        };
        ui.add_space(6.0);
        ui.label(format!(
            "Live {}   BPM {}   {} 条轨道   {} 个 Clip",
            report.minor_version,
            report
                .bpm
                .map(blacklive::format_num)
                .unwrap_or_else(|| "?".into()),
            report.tracks.len(),
            report.clips.len()
        ));

        if report.clips.is_empty() {
            ui.colored_label(Color32::from_rgb(240, 170, 80), "模板里没有 MidiClip。");
        } else {
            let mut selected = self.selected_clip;
            for clip in &report.clips {
                let label = format!("#{}  {}", clip.candidate_index, clip.display_label());
                ui.selectable_value(&mut selected, Some(clip.candidate_index), label);
            }
            self.selected_clip = selected;
        }

        egui::CollapsingHeader::new("轨道")
            .default_open(false)
            .show(ui, |ui| {
                for track in &report.tracks {
                    ui.label(format!(
                        "#{} {}{}",
                        track.index,
                        track.name,
                        if track.has_instrument {
                            ""
                        } else {
                            "  [!] 无乐器"
                        }
                    ));
                    if !track.devices.is_empty() {
                        ui.label(format!("    设备: {}", track.devices.join(", ")));
                    }
                }
            });

        if !report.warnings.is_empty() {
            ui.add_space(2.0);
            for warning in &report.warnings {
                ui.label(
                    RichText::new(format!("⚠ {warning}")).color(Color32::from_rgb(235, 180, 90)),
                );
            }
        }
    }

    fn show_midi_stats(&mut self, ui: &mut egui::Ui) {
        let Some(stats) = &self.midi_stats else {
            return;
        };
        ui.add_space(6.0);
        ui.label(format!(
            "{} 音符   PPQ {}   BPM {}   最大复音 {}   {} beats",
            stats.note_count,
            stats.ppq,
            stats
                .initial_bpm
                .map(blacklive::format_num)
                .unwrap_or_else(|| "?".into()),
            stats.max_polyphony,
            blacklive::format_num(stats.duration_beats)
        ));
        ui.horizontal_wrapped(|ui| {
            for (channel, count) in &stats.notes_per_channel {
                ui.label(format!("ch{}: {count}", channel));
            }
        });
        if stats.sustain_event_count > 0 {
            ui.label(format!("CC64: {}", stats.sustain_event_count));
        }
        if stats.tempo_map.len() > 1 {
            ui.label(format!("Tempo 点: {}", stats.tempo_map.len()));
        }
    }

    fn show_options(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        egui::Grid::new("options")
            .num_columns(4)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("通道");
                ui.add(egui::TextEdit::singleline(&mut self.channels).desired_width(150.0));
                ui.checkbox(&mut self.include_drum_channel, "包含 GM 鼓通道");
                ui.end_row();

                ui.label("移调");
                ui.add(egui::DragValue::new(&mut self.transpose).suffix(" 半音"));
                ui.label("BPM");
                ui.add(egui::TextEdit::singleline(&mut self.bpm_text).desired_width(90.0));
                ui.end_row();

                ui.label("最短时值");
                ui.add(
                    egui::DragValue::new(&mut self.min_note_beats)
                        .speed(0.001)
                        .suffix(" beats"),
                );
                ui.label("尾部留白");
                ui.add(
                    egui::DragValue::new(&mut self.tail_beats)
                        .speed(0.01)
                        .suffix(" beats"),
                );
                ui.end_row();

                ui.label("Arrangement 起点");
                ui.add(
                    egui::DragValue::new(&mut self.start_beats)
                        .speed(0.25)
                        .suffix(" beats"),
                );
                ui.checkbox(&mut self.trim_same_pitch, "同音高重叠截短前音");
                ui.end_row();

                ui.label("Clip 名称");
                ui.add(egui::TextEdit::singleline(&mut self.clip_name).desired_width(150.0));
                ui.label("轨道名称");
                ui.add(egui::TextEdit::singleline(&mut self.track_name).desired_width(150.0));
                ui.end_row();
            });
    }

    fn show_actions(&mut self, ui: &mut egui::Ui) -> ActionFlags {
        let mut flags = ActionFlags::default();
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let can_convert =
                !self.midi_path.trim().is_empty() && !self.template_path.trim().is_empty();
            if ui
                .add_enabled(
                    can_convert,
                    egui::Button::new(RichText::new("转换并生成 .als").strong()),
                )
                .clicked()
            {
                flags.convert = true;
            }
            if let Some(result) = &self.last_result {
                if ui.button("在 Live 中打开").clicked() {
                    flags.open_result = Some(result.output_path.clone());
                }
                if ui.button("打开所在文件夹").clicked() {
                    flags.open_folder = Some(result.output_path.clone());
                }
            }
        });
        flags
    }

    fn show_status(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        let color = if self.status_is_error {
            Color32::from_rgb(245, 110, 110)
        } else {
            Color32::from_rgb(120, 210, 150)
        };
        ui.label(RichText::new(&self.status).color(color).strong());

        if let Some(result) = &self.last_result {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "写入 {} / 源 {} 音符   BPM {}   长度 {} beats",
                    result.used_note_count,
                    result.source_note_count,
                    blacklive::format_num(result.bpm),
                    blacklive::format_num(result.clip_length_beats)
                ));
                if result.tempo_automation {
                    ui.label(format!(
                        "Tempo 自动化 {} 点",
                        result.midi_stats.tempo_map.len()
                    ));
                }
            });
        }

        if !self.log.is_empty() {
            ui.add_space(2.0);
            egui::ScrollArea::vertical()
                .id_salt("log_scroll")
                .max_height(150.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.log {
                        let color = if line.starts_with('!') {
                            Color32::from_rgb(245, 110, 110)
                        } else if line.starts_with('⚠') {
                            Color32::from_rgb(235, 180, 90)
                        } else {
                            Color32::GRAY
                        };
                        ui.label(RichText::new(line).color(color));
                    }
                });
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_dropped_files(ui.ctx());

        let mut top_actions = FileActions::default();
        let mut action_flags = ActionFlags::default();

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Blacklive");
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("main_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    top_actions = self.show_files(ui);
                    self.show_template(ui);
                    self.show_midi_stats(ui);
                    self.show_options(ui);
                    action_flags = self.show_actions(ui);
                    self.show_status(ui);
                });
        });

        if top_actions.choose_midi {
            self.choose_midi();
        }
        if top_actions.choose_template {
            self.choose_template();
        }
        if top_actions.choose_output {
            self.choose_output();
        }
        if top_actions.analyze_template {
            self.analyze_template();
        }
        if top_actions.analyze_midi {
            self.analyze_midi();
        }
        if top_actions.dump_xml {
            self.dump_xml();
        }
        if action_flags.convert {
            self.convert();
        }
        if let Some(path) = action_flags.open_result
            && let Err(err) = open_in_default(&path)
        {
            self.set_error(format!("{err:#}"));
        }
        if let Some(path) = action_flags.open_folder
            && let Err(err) = reveal_in_folder(&path)
        {
            self.set_error(format!("{err:#}"));
        }
    }
}

impl App {
    fn dump_xml(&mut self) {
        let template = self.template_path.trim();
        if template.is_empty() {
            self.set_error("请先选择模板 .als");
            return;
        }
        let default_name = Path::new(template)
            .with_extension("xml")
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "template.xml".into());
        let Some(output) = rfd::FileDialog::new()
            .add_filter("XML", &["xml"])
            .set_file_name(default_name)
            .save_file()
        else {
            return;
        };
        match blacklive::als::dump_template_xml(Path::new(template), &output) {
            Ok(()) => {
                self.log(format!("导出模板 XML: {}", output.display()));
                self.set_status(format!("已导出 XML：{}", output.display()));
            }
            Err(err) => self.set_error(format!("{err:#}")),
        }
    }
}

#[derive(Default)]
struct FileActions {
    choose_midi: bool,
    choose_template: bool,
    choose_output: bool,
    analyze_template: bool,
    analyze_midi: bool,
    dump_xml: bool,
}

#[derive(Default)]
struct ActionFlags {
    convert: bool,
    open_result: Option<PathBuf>,
    open_folder: Option<PathBuf>,
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn open_in_default(path: &Path) -> Result<()> {
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

fn reveal_in_folder(path: &Path) -> Result<()> {
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
