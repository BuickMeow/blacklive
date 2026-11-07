use std::path::{Path, PathBuf};

use blacklive::{
    ConversionResult, ConvertOptions, MidiStats, TemplateReport, convert_midi_to_als,
    convert_midi_to_als_with_default_template, parse_channel_filter,
};
use eframe::egui::{self, RichText};

use super::actions::{ActionFlags, FileActions, non_empty, open_in_default, reveal_in_folder};

pub(crate) struct App {
    pub(crate) midi_path: String,
    pub(crate) template_path: String,
    pub(crate) output_path: String,

    pub(crate) channels: String,
    pub(crate) include_drum_channel: bool,
    pub(crate) transpose: i32,
    pub(crate) bpm_text: String,
    pub(crate) min_note_beats: f64,
    pub(crate) tail_beats: f64,
    pub(crate) start_beats: f64,
    pub(crate) trim_same_pitch: bool,
    pub(crate) clip_name: String,
    pub(crate) split_source_tracks: bool,
    pub(crate) strip_instrument_effects: bool,

    pub(crate) template_report: Option<TemplateReport>,
    pub(crate) selected_clip: Option<usize>,
    pub(crate) midi_stats: Option<MidiStats>,
    pub(crate) last_result: Option<ConversionResult>,

    pub(crate) status: String,
    pub(crate) status_is_error: bool,
    pub(crate) log: Vec<String>,
}

impl App {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
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
            split_source_tracks: true,
            strip_instrument_effects: true,
            template_report: None,
            selected_clip: None,
            midi_stats: None,
            last_result: None,
            status: "选择一个 MIDI 文件".to_string(),
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

    pub(crate) fn set_status(&mut self, message: impl Into<String>) {
        self.status = message.into();
        self.status_is_error = false;
    }

    pub(crate) fn set_error(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.status = message.clone();
        self.status_is_error = true;
        self.log.push(format!("! {message}"));
    }

    pub(crate) fn log(&mut self, message: impl Into<String>) {
        self.log.push(message.into());
    }

    pub(crate) fn convert(&mut self) {
        let midi = PathBuf::from(self.midi_path.trim());
        if midi.as_os_str().is_empty() {
            self.set_error("请先选择 MIDI");
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
            template_clip_index: if self.template_is_builtin() {
                None
            } else {
                self.selected_clip
            },
            template_clip_name: None,
            channels,
            include_drum_channel: self.include_drum_channel,
            transpose: self.transpose,
            bpm_override,
            min_note_beats: self.min_note_beats,
            tail_beats: self.tail_beats,
            start_beats: self.start_beats,
            clip_name: non_empty(&self.clip_name),
            trim_same_pitch: self.trim_same_pitch,
            split_source_tracks: self.split_source_tracks,
            keep_empty_tracks: true,
            write_pitch_bend: true,
            pitch_bend_range: None,
            write_cc_automation: false,
            strip_instrument_effects: self.strip_instrument_effects,
        };

        self.set_status("正在转换…");
        let converted = if self.template_is_builtin() {
            convert_midi_to_als_with_default_template(&midi, &output, &options)
        } else {
            convert_midi_to_als(
                &midi,
                Path::new(self.template_path.trim()),
                &output,
                &options,
            )
        };
        match converted {
            Ok(result) => {
                self.log(format!(
                    "✓ 使用{}输出 {}，写入 {} 音符，{} 组 / {} MIDI 轨，BPM {}，Clip {:.2} beats",
                    result.template_label,
                    result.output_path.display(),
                    result.used_note_count,
                    result.groups.len(),
                    result.groups.iter().map(|g| g.tracks.len()).sum::<usize>(),
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
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_dropped_files(ui.ctx());

        let mut primary_files = FileActions::default();
        let mut advanced_files = FileActions::default();
        let mut action_flags = ActionFlags::default();

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Blacklive");
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("main_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let can_export = !self.midi_path.trim().is_empty();
                    ui.horizontal(|ui| {
                        if ui.button("选择 MIDI…").clicked() {
                            primary_files.choose_midi = true;
                        }
                        if ui
                            .add_enabled(
                                can_export,
                                egui::Button::new(RichText::new("导出 .als").strong()),
                            )
                            .clicked()
                        {
                            action_flags.convert = true;
                        }
                        if let Some(result) = &self.last_result {
                            if ui.button("在 Live 中打开").clicked() {
                                action_flags.open_result = Some(result.output_path.clone());
                            }
                            if ui.button("打开所在文件夹").clicked() {
                                action_flags.open_folder = Some(result.output_path.clone());
                            }
                        }
                    });

                    let midi = self.midi_path.trim();
                    if midi.is_empty() {
                        ui.label("未选择 MIDI");
                    } else {
                        ui.label(format!("MIDI  {midi}"));
                        let output = if self.output_path.trim().is_empty() {
                            PathBuf::from(midi).with_extension("als")
                        } else {
                            PathBuf::from(self.output_path.trim())
                        };
                        ui.label(format!("输出  {}", output.display()));
                    }

                    self.show_status(ui);

                    egui::CollapsingHeader::new("高级")
                        .default_open(false)
                        .show(ui, |ui| {
                            advanced_files = self.show_files(ui);
                            self.show_template(ui);
                            self.show_midi_stats(ui);
                            self.show_options(ui);
                        });
                });
        });

        if primary_files.choose_midi || advanced_files.choose_midi {
            self.choose_midi();
        }
        if advanced_files.choose_template {
            self.choose_template();
        }
        if advanced_files.clear_template {
            self.clear_template();
        }
        if advanced_files.choose_output {
            self.choose_output();
        }
        if advanced_files.analyze_template {
            self.analyze_template();
        }
        if advanced_files.analyze_midi {
            self.analyze_midi();
        }
        if advanced_files.dump_xml {
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
