use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};

use blacklive::{
    ConversionResult, ConvertOptions, MidiStats, TemplateReport, convert_midi_to_als,
    convert_midi_to_als_with_default_template, parse_channel_filter,
};
use eframe::egui::{self, CursorIcon, ViewportCommand};

use super::actions::UiActions;
use super::project::{ProjectView, ViewMode};
use super::theme;
use super::{arrangement, browser, chrome, detail, session};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DetailTab {
    Convert,
    Info,
    Log,
}

pub(crate) struct App {
    pub(crate) midi_path: String,
    pub(crate) template_path: String,
    pub(crate) output_path: String,

    pub(crate) channels: String,
    pub(crate) include_drum_channel: bool,
    pub(crate) transpose: i32,
    pub(crate) bpm_text: String,
    pub(crate) bpm_edit: Option<String>,
    pub(crate) bpm_focus: bool,
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

    pub(crate) view_mode: ViewMode,
    pub(crate) detail_tab: DetailTab,
    pub(crate) detail_height: f32,
    pub(crate) selected_track: Option<usize>,
    pub(crate) armed_tracks: std::collections::BTreeSet<usize>,
    pub(crate) muted_tracks: std::collections::BTreeSet<usize>,
    pub(crate) solo_tracks: std::collections::BTreeSet<usize>,

    pub(crate) browser_search: String,
    pub(crate) browser_section: String,
    pub(crate) browser_tag: String,
    pub(crate) browser_filters: bool,

    pub(crate) playing: bool,
    pub(crate) playhead_beats: f64,
    pub(crate) arr_scroll_y: f32,
    pub(crate) h_offset_bars: f64,
    pub(crate) session_scroll_x: f32,
    pub(crate) last_tick: Option<std::time::Instant>,
    pub(crate) loop_enabled: bool,
    pub(crate) link_enabled: bool,
    pub(crate) metronome_on: bool,
    pub(crate) tap_times: Vec<std::time::Instant>,
    pub(crate) midi_parse_rx: Option<Receiver<Result<MidiStats, String>>>,
    pub(crate) window_title: String,
}

impl App {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(egui::ThemePreference::Light);
        theme::apply(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        let mut app = Self {
            midi_path: String::new(),
            template_path: String::new(),
            output_path: String::new(),
            channels: String::new(),
            include_drum_channel: false,
            transpose: 0,
            bpm_text: String::new(),
            bpm_edit: None,
            bpm_focus: false,
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
            status: "选择一个 MIDI 文件，或把 .mid / .midi 拖进窗口".to_string(),
            status_is_error: false,
            log: Vec::new(),
            view_mode: if std::env::var("BLACKLIVE_VIEW").as_deref() == Ok("session") {
                ViewMode::Session
            } else {
                ViewMode::Arrangement
            },
            detail_tab: DetailTab::Convert,
            detail_height: 188.0,
            selected_track: Some(0),
            armed_tracks: Default::default(),
            muted_tracks: Default::default(),
            solo_tracks: Default::default(),
            browser_search: String::new(),
            browser_section: "当前工程".to_string(),
            browser_tag: "全部".to_string(),
            browser_filters: true,
            playing: false,
            playhead_beats: 0.0,
            arr_scroll_y: 0.0,
            h_offset_bars: 0.0,
            session_scroll_x: 0.0,
            last_tick: None,
            loop_enabled: true,
            link_enabled: false,
            metronome_on: false,
            tap_times: Vec::new(),
            midi_parse_rx: None,
            window_title: String::new(),
        };

        let args: Vec<String> = std::env::args().skip(1).collect();
        if let Some(path) = args.first() {
            app.midi_path = path.clone();
            app.output_path = PathBuf::from(path)
                .with_extension("als")
                .display()
                .to_string();
            app.start_parse(cc.egui_ctx.clone());
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

    pub(crate) fn effective_bpm(&self) -> f64 {
        self.bpm_text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|bpm| bpm.is_finite() && *bpm > 0.0)
            .or(self.midi_stats.as_ref().and_then(|stats| stats.initial_bpm))
            .or(self.last_result.as_ref().map(|result| result.bpm))
            .or(self.template_report.as_ref().and_then(|report| report.bpm))
            .unwrap_or(120.0)
    }

    pub(crate) fn project(&self) -> ProjectView {
        let bpm_override = self
            .bpm_text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|bpm| bpm.is_finite() && *bpm > 0.0);
        let mut view = super::project::view_from(
            &self.midi_path,
            self.midi_stats.as_ref(),
            self.last_result.as_ref(),
            bpm_override,
            self.start_beats,
        );
        for (index, track) in view.tracks.iter_mut().enumerate() {
            track.armed = self.armed_tracks.contains(&index);
            track.muted = self.muted_tracks.contains(&index);
            track.solo = self.solo_tracks.contains(&index);
        }
        view
    }

    pub(crate) fn begin_bpm_edit(&mut self) {
        let bpm = self.effective_bpm();
        self.bpm_edit = Some(format!("{bpm:.2}"));
        self.bpm_focus = true;
    }

    pub(crate) fn commit_bpm_edit(&mut self) {
        let Some(buffer) = self.bpm_edit.take() else {
            return;
        };
        let value = buffer.trim();
        if value.is_empty() {
            self.bpm_text.clear();
            self.set_status("BPM 跟随 MIDI / 模板");
            return;
        }
        match value.parse::<f64>() {
            Ok(bpm) if bpm.is_finite() && bpm > 0.0 => {
                self.bpm_text = format!("{bpm:.2}");
                self.set_status(format!("BPM 覆盖为 {}", blacklive::format_num(bpm)));
            }
            _ => self.set_error(format!("BPM「{value}」不是正数")),
        }
    }

    pub(crate) fn tap_tempo(&mut self) {
        let now = std::time::Instant::now();
        self.tap_times
            .retain(|tap| now.duration_since(*tap).as_secs_f64() < 2.0);
        self.tap_times.push(now);
        if self.tap_times.len() >= 2 {
            let intervals: Vec<f64> = self
                .tap_times
                .windows(2)
                .map(|pair| pair[1].duration_since(pair[0]).as_secs_f64())
                .filter(|seconds| *seconds > 0.15)
                .collect();
            if !intervals.is_empty() {
                let average = intervals.iter().sum::<f64>() / intervals.len() as f64;
                let bpm = 60.0 / average;
                if bpm.is_finite() && bpm > 0.0 {
                    self.bpm_text = format!("{bpm:.2}");
                    self.set_status(format!("Tap BPM {}", blacklive::format_num(bpm)));
                }
            }
        }
    }

    pub(crate) fn toggle_play(&mut self) {
        if self.playing {
            self.playing = false;
            self.last_tick = None;
        } else {
            self.playing = true;
            self.last_tick = Some(std::time::Instant::now());
        }
    }

    pub(crate) fn stop(&mut self) {
        self.playing = false;
        self.playhead_beats = 0.0;
        self.last_tick = None;
    }

    pub(crate) fn toggle_record_arm(&mut self) {
        let index = self.selected_track.unwrap_or(0);
        if !self.armed_tracks.insert(index) {
            self.armed_tracks.remove(&index);
        }
        let armed = self.armed_tracks.contains(&index);
        self.set_status(if armed {
            format!("轨道 {} 已预备录制", index + 1)
        } else {
            format!("轨道 {} 已解除录制预备", index + 1)
        });
    }

    pub(crate) fn advance_transport(&mut self, ctx: &egui::Context) {
        if !self.playing {
            self.last_tick = None;
            return;
        }
        let now = std::time::Instant::now();
        if let Some(last) = self.last_tick {
            let dt = now.duration_since(last).as_secs_f64();
            self.playhead_beats += dt * self.effective_bpm() / 60.0;
        }
        self.last_tick = Some(now);
        if self.loop_enabled && self.playhead_beats >= 16.0 {
            self.playhead_beats %= 16.0;
        }
        ctx.request_repaint();
    }

    pub(crate) fn start_parse(&mut self, ctx: egui::Context) {
        let path = self.midi_path.trim().to_string();
        if path.is_empty() {
            return;
        }
        self.midi_stats = None;
        self.last_result = None;
        self.playhead_beats = 0.0;
        self.playing = false;
        self.status = "正在解析 MIDI…".to_string();
        self.status_is_error = false;
        let (tx, rx) = std::sync::mpsc::channel();
        self.midi_parse_rx = Some(rx);
        std::thread::spawn(move || {
            let result = blacklive::parse_midi_file(Path::new(&path))
                .map(|parsed| parsed.stats)
                .map_err(|err| format!("{err:#}"));
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }

    pub(crate) fn poll_parse(&mut self) {
        let Some(rx) = &self.midi_parse_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(stats)) => {
                self.log(format!(
                    "MIDI 解析完成：{} 音符，{} 通道，PPQ {}",
                    stats.note_count,
                    stats.notes_per_channel.len(),
                    stats.ppq
                ));
                let channels = stats.notes_per_channel.len();
                self.midi_stats = Some(stats);
                self.midi_parse_rx = None;
                self.selected_track = Some(0);
                self.set_status(format!("MIDI 已载入 · {channels} 个通道"));
            }
            Ok(Err(error)) => {
                self.midi_parse_rx = None;
                self.set_error(error);
            }
            Err(TryRecvError::Disconnected) => {
                self.midi_parse_rx = None;
                self.set_error("MIDI 解析线程意外结束");
            }
            Err(TryRecvError::Empty) => {}
        }
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
                self.detail_tab = DetailTab::Info;
            }
            Err(err) => {
                self.last_result = None;
                self.set_error(format!("{err:#}"));
            }
        }
    }

    fn apply_actions(&mut self, actions: UiActions, ctx: &egui::Context) {
        if let Some(view) = actions.set_view {
            self.view_mode = view;
        }
        if actions.choose_midi {
            self.choose_midi(ctx.clone());
        }
        if actions.choose_template {
            self.choose_template();
        }
        if actions.clear_template {
            self.clear_template();
        }
        if actions.choose_output {
            self.choose_output();
        }
        if actions.analyze_template {
            self.analyze_template();
        }
        if actions.analyze_midi {
            self.analyze_midi();
        }
        if actions.dump_xml {
            self.dump_xml();
        }
        if actions.convert {
            self.convert();
        }
        if let Some(path) = actions.open_result
            && let Err(err) = open_in_default(&path)
        {
            self.set_error(format!("{err:#}"));
        }
        if let Some(path) = actions.open_folder
            && let Err(err) = reveal_in_folder(&path)
        {
            self.set_error(format!("{err:#}"));
        }
    }

    fn update_window_title(&mut self, ctx: &egui::Context, project: &ProjectView) {
        let title = if project.placeholder {
            "Blacklive".to_string()
        } else {
            format!("{} — Blacklive", project.title)
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_dropped_files(ui.ctx());
        self.poll_parse();
        self.advance_transport(ui.ctx());

        let mut actions = UiActions::default();
        let project = self.project();
        let detail_height = self.detail_height;
        let view = self.view_mode;

        egui::Panel::top("blacklive_title")
            .exact_size(28.0)
            .show_separator_line(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                chrome::title_bar(ui, self, &mut actions);
            });
        egui::Panel::top("blacklive_transport")
            .exact_size(30.0)
            .show_separator_line(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                chrome::transport_bar(ui, self, &project, &mut actions);
            });
        egui::Panel::bottom("blacklive_status")
            .exact_size(34.0)
            .show_separator_line(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                chrome::status_bar(ui, self);
            });
        egui::Panel::bottom("blacklive_detail")
            .exact_size(detail_height)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                detail::show(ui, self, &project, &mut actions);
            });
        egui::Panel::left("blacklive_browser")
            .exact_size(theme::BROWSER_W)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                browser::show(ui, self, &project, &mut actions);
            });
        egui::CentralPanel::no_frame().show(ui, |ui| match view {
            ViewMode::Arrangement => arrangement::show(ui, self, &project, &mut actions),
            ViewMode::Session => session::show(ui, self, &project, &mut actions),
        });

        self.apply_actions(actions, ui.ctx());
        self.update_window_title(ui.ctx(), &project);
        resize_handles(ui.ctx());
        ui.painter().rect_stroke(
            ui.max_rect().shrink(0.5),
            egui::CornerRadius::ZERO,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(0x4F, 0x4F, 0x4F)),
            egui::StrokeKind::Inside,
        );
    }
}

fn resize_handles(ctx: &egui::Context) {
    let Some(pos) = ctx.pointer_latest_pos() else {
        return;
    };
    let screen = ctx.content_rect();
    let edge = 4.0;
    let near_left = pos.x <= screen.left() + edge;
    let near_right = pos.x >= screen.right() - edge;
    let near_top = pos.y <= screen.top() + edge;
    let near_bottom = pos.y >= screen.bottom() - edge;
    let direction = match (near_top, near_bottom, near_left, near_right) {
        (true, _, true, _) => Some(egui::ResizeDirection::NorthWest),
        (true, _, _, true) => Some(egui::ResizeDirection::NorthEast),
        (_, true, true, _) => Some(egui::ResizeDirection::SouthWest),
        (_, true, _, true) => Some(egui::ResizeDirection::SouthEast),
        (true, _, _, _) => Some(egui::ResizeDirection::North),
        (_, true, _, _) => Some(egui::ResizeDirection::South),
        (_, _, true, _) => Some(egui::ResizeDirection::West),
        (_, _, _, true) => Some(egui::ResizeDirection::East),
        _ => None,
    };
    let Some(direction) = direction else {
        return;
    };
    ctx.set_cursor_icon(match direction {
        egui::ResizeDirection::North | egui::ResizeDirection::South => CursorIcon::ResizeVertical,
        egui::ResizeDirection::East | egui::ResizeDirection::West => CursorIcon::ResizeHorizontal,
        egui::ResizeDirection::NorthEast | egui::ResizeDirection::SouthWest => {
            CursorIcon::ResizeNeSw
        }
        egui::ResizeDirection::NorthWest | egui::ResizeDirection::SouthEast => {
            CursorIcon::ResizeNwSe
        }
    });
    if ctx.input(|input| input.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
    }
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

use super::actions::{open_in_default, reveal_in_folder};
