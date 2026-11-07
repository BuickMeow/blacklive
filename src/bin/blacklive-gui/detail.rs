use eframe::egui::{self, Align, Align2, Color32, CursorIcon, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use egui_extras::{Column, TableBuilder};

use super::App;
use super::actions::UiActions;
use super::icons;
use super::project::ProjectView;
use super::star_grid::{StarGrid, TrackSize, measure_text};
use super::theme;
use crate::app::DetailTab;

const TAB_H: f32 = 22.0;

pub(crate) fn show(ui: &mut Ui, app: &mut App, project: &ProjectView, actions: &mut UiActions) {
    let rect = ui.max_rect();
    let painter = ui.painter().clone();
    painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE);
    painter.hline(
        rect.x_range(),
        rect.top() + 0.5,
        Stroke::new(1.0, theme::BORDER),
    );
    painter.hline(
        rect.x_range(),
        rect.top() + 1.5,
        Stroke::new(1.0, theme::PANEL_HI),
    );

    // Resize handle on the top border.
    let handle = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 4.0));
    let resize = ui.interact(
        handle,
        ui.make_persistent_id("detail_resize"),
        Sense::drag(),
    );
    if resize.dragged() {
        app.detail_height = (app.detail_height - resize.drag_delta().y).clamp(140.0, 380.0);
    }
    if resize.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
    }

    // Left grip, like Live's detail-view handle.
    let grip = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + TAB_H),
        Pos2::new(rect.left() + 9.0, rect.bottom()),
    );
    painter.rect_filled(
        grip,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0x8C, 0x8C, 0x8C),
    );
    for i in 0..14 {
        painter.circle_filled(
            Pos2::new(grip.center().x, grip.top() + 16.0 + i as f32 * 10.0),
            1.6,
            Color32::from_rgb(0x63, 0x63, 0x63),
        );
    }
    painter.vline(
        grip.right() - 0.5,
        grip.y_range(),
        Stroke::new(1.0, theme::BORDER_SOFT),
    );

    // Tabs.
    let tabs = [
        ("转换", icons::TUNE, DetailTab::Convert),
        ("信息", icons::INFO, DetailTab::Info),
        ("日志", icons::GRAPHIC_EQ, DetailTab::Log),
    ];
    let mut x = rect.left() + 18.0;
    for (label, icon, tab) in tabs {
        let tab_rect = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(64.0, TAB_H));
        let active = app.detail_tab == tab;
        let response = ui.interact(
            tab_rect,
            ui.make_persistent_id(("detail_tab", label)),
            Sense::click(),
        );
        if active {
            painter.rect_filled(
                tab_rect,
                egui::CornerRadius::ZERO,
                Color32::from_rgb(0xBE, 0xBE, 0xBE),
            );
            painter.hline(
                tab_rect.x_range(),
                tab_rect.top() + 0.5,
                Stroke::new(2.0, theme::ACCENT_DARK),
            );
        } else if response.hovered() {
            painter.rect_filled(
                tab_rect,
                egui::CornerRadius::ZERO,
                Color32::from_rgb(0xB2, 0xB2, 0xB2),
            );
        }
        painter.vline(
            tab_rect.right() - 0.5,
            tab_rect.y_range(),
            Stroke::new(1.0, theme::BORDER_SOFT),
        );
        theme::mixed_icon_label(
            &painter,
            tab_rect,
            icon,
            label,
            10.0,
            if active { theme::TEXT } else { theme::TEXT_DIM },
        );
        if response.clicked() {
            app.detail_tab = tab;
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        x += 64.0;
    }
    painter.hline(
        rect.x_range(),
        rect.top() + TAB_H - 0.5,
        Stroke::new(1.0, theme::BORDER_SOFT),
    );

    let content = Rect::from_min_max(
        Pos2::new(rect.left() + 12.0, rect.top() + TAB_H + 4.0),
        Pos2::new(rect.right() - 8.0, rect.bottom() - 6.0),
    );
    match app.detail_tab {
        DetailTab::Convert => convert_tab(ui, app, project, content, actions),
        DetailTab::Info => info_tab(ui, app, project, content),
        DetailTab::Log => log_tab(ui, app, content),
    }
}

fn convert_tab(
    ui: &mut Ui,
    app: &mut App,
    project: &ProjectView,
    rect: Rect,
    actions: &mut UiActions,
) {
    let left_w = (rect.width() - 320.0).clamp(520.0, 780.0);
    let device = Rect::from_min_size(rect.min, Vec2::new(left_w, rect.height()));
    draw_converter_device(ui, app, project, device);

    let right_x = device.right() + 10.0;
    let export = Rect::from_min_size(
        Pos2::new(right_x, rect.top()),
        Vec2::new((rect.right() - right_x).min(300.0), 88.0),
    );
    draw_export_device(ui, app, export, actions);

    if export.bottom() + 8.0 < rect.bottom() {
        let template = Rect::from_min_max(
            Pos2::new(right_x, export.bottom() + 8.0),
            Pos2::new(export.right(), rect.bottom()),
        );
        draw_template_device(ui, app, template, actions);
    }
}

fn draw_converter_device(ui: &mut Ui, app: &mut App, project: &ProjectView, rect: Rect) {
    theme::shadowed_device(ui.painter(), rect);
    let header = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 20.0));
    ui.painter().rect_filled(
        header,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0xBE, 0xBE, 0xBE),
    );
    ui.painter().hline(
        header.x_range(),
        header.bottom() - 0.5,
        Stroke::new(1.0, theme::BORDER),
    );
    ui.painter().rect_filled(
        Rect::from_min_size(header.min, Vec2::new(4.0, header.height())),
        egui::CornerRadius::ZERO,
        theme::ACCENT_DARK,
    );
    ui.painter().text(
        Pos2::new(header.left() + 12.0, header.center().y),
        Align2::LEFT_CENTER,
        icons::TUNE,
        icons::font(12.0),
        theme::TEXT,
    );
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(header.left() + 28.0, header.top()),
            Pos2::new(header.right() - 150.0, header.bottom()),
        ),
        "Blacklive",
        10.5,
        theme::TEXT,
        Align2::LEFT_CENTER,
    );
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(header.left() + 90.0, header.top()),
            Pos2::new(header.right() - 120.0, header.bottom()),
        ),
        "MIDI → Ableton .als",
        9.0,
        theme::TEXT_DIM,
        Align2::LEFT_CENTER,
    );
    let led_center = Pos2::new(header.right() - 38.0, header.center().y);
    ui.painter()
        .circle_filled(led_center, 3.5, theme::ACCENT_FIELD);
    ui.painter().circle_stroke(
        led_center,
        3.5,
        Stroke::new(0.8, Color32::from_rgb(0x6B, 0x52, 0x00)),
    );
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(led_center.x + 7.0, header.top()),
            Pos2::new(header.right() - 6.0, header.bottom()),
        ),
        "On",
        8.5,
        theme::TEXT,
        Align2::LEFT_CENTER,
    );
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(header.right() - 112.0, header.top()),
            Pos2::new(header.right() - 50.0, header.bottom()),
        ),
        &if project.total_notes == 0 {
            "等待 MIDI".to_string()
        } else {
            format!("{} 音符", project.total_notes)
        },
        8.5,
        theme::TEXT_DIM,
        Align2::RIGHT_CENTER,
    );

    let body = Rect::from_min_max(
        Pos2::new(rect.left() + 7.0, header.bottom() + 5.0),
        Pos2::new(rect.right() - 7.0, rect.bottom() - 5.0),
    );
    let labels = [
        "BPM",
        "移调",
        "通道",
        "Clip",
        "最短时值",
        "尾部",
        "起点",
        "Pitch Bend",
        "拆分",
        "效果",
        "鼓",
        "重叠",
    ];
    let auto_width = labels
        .iter()
        .map(|label| measure_text(ui, label, 9.5))
        .fold(0.0, f32::max)
        + 8.0;
    let grid = StarGrid::new(
        vec![
            TrackSize::Auto,
            TrackSize::Star(2.0),
            TrackSize::Auto,
            TrackSize::Star(2.0),
            TrackSize::Star(1.0),
        ],
        vec![TrackSize::Fixed(19.0); 6],
    )
    .gaps(6.0, 1.0);
    let cells = grid.cells(body, &[auto_width, 0.0, auto_width, 0.0, 0.0], &[]);
    let cell = |row: usize, col: usize| cells[row][col];

    let label = |ui: &Ui, rect: Rect, text: &str| {
        theme::label_at(
            ui.painter(),
            rect,
            text,
            9.5,
            theme::TEXT_DIM,
            Align2::LEFT_CENTER,
        )
    };

    label(ui, cell(0, 0), "BPM");
    theme::text_edit_at(
        ui,
        cell(0, 1),
        &mut app.bpm_text,
        "跟随 MIDI",
        10.0,
        Align::Center,
    );
    label(ui, cell(0, 2), "移调");
    theme::drag_i32_at(ui, cell(0, 3), &mut app.transpose, " 半音", -48..=48);

    label(ui, cell(1, 0), "通道");
    theme::text_edit_at(
        ui,
        cell(1, 1),
        &mut app.channels,
        "全部",
        10.0,
        Align::Center,
    );
    label(ui, cell(1, 2), "Clip");
    theme::text_edit_at(
        ui,
        cell(1, 3),
        &mut app.clip_name,
        "自动命名",
        10.0,
        Align::Center,
    );

    label(ui, cell(2, 0), "最短时值");
    theme::drag_value_at(
        ui,
        cell(2, 1),
        &mut app.min_note_beats,
        " beats",
        0.01,
        0.0..=64.0,
        3,
    );
    label(ui, cell(2, 2), "尾部");
    theme::drag_value_at(
        ui,
        cell(2, 3),
        &mut app.tail_beats,
        " beats",
        0.1,
        0.0..=128.0,
        2,
    );

    label(ui, cell(3, 0), "起点");
    theme::drag_value_at(
        ui,
        cell(3, 1),
        &mut app.start_beats,
        " beats",
        0.25,
        0.0..=4096.0,
        2,
    );
    label(ui, cell(3, 2), "Pitch Bend");
    theme::label_at(
        ui.painter(),
        cell(3, 3),
        "自动 · RPN 折算",
        9.5,
        theme::TEXT,
        Align2::LEFT_CENTER,
    );

    label(ui, cell(4, 0), "拆分");
    theme::toggle_at(ui, cell(4, 1), &mut app.split_source_tracks, "按源轨道拆分");
    label(ui, cell(4, 2), "效果");
    theme::toggle_at(
        ui,
        cell(4, 3),
        &mut app.strip_instrument_effects,
        "关闭乐器内效果",
    );

    label(ui, cell(5, 0), "鼓");
    theme::toggle_at(ui, cell(5, 1), &mut app.include_drum_channel, "包含 GM 鼓");
    label(ui, cell(5, 2), "重叠");
    theme::toggle_at(ui, cell(5, 3), &mut app.trim_same_pitch, "同音高截短");
}

fn draw_export_device(ui: &mut Ui, app: &mut App, rect: Rect, actions: &mut UiActions) {
    theme::shadowed_device(ui.painter(), rect);
    let header = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 18.0));
    ui.painter().rect_filled(
        header,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0xBE, 0xBE, 0xBE),
    );
    ui.painter().hline(
        header.x_range(),
        header.bottom() - 0.5,
        Stroke::new(1.0, theme::BORDER),
    );
    ui.painter().text(
        Pos2::new(header.left() + 8.0, header.center().y),
        Align2::LEFT_CENTER,
        icons::SAVE,
        icons::font(11.0),
        theme::TEXT,
    );
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(header.left() + 24.0, header.top()),
            Pos2::new(header.right() - 6.0, header.bottom()),
        ),
        "导出",
        10.0,
        theme::TEXT,
        Align2::LEFT_CENTER,
    );

    let can_export = !app.midi_path.trim().is_empty();
    let export_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 6.0, header.bottom() + 4.0),
        Vec2::new(rect.width() - 12.0, 26.0),
    );
    if theme::bevel_button(
        ui,
        export_rect,
        "导出 .als",
        Some(icons::UPLOAD),
        can_export,
        can_export,
    )
    .clicked()
    {
        actions.convert = true;
    }

    let gap = 4.0;
    let bw = (rect.width() - 12.0 - gap) * 0.5;
    let row1 = Rect::from_min_size(
        Pos2::new(rect.left() + 6.0, export_rect.bottom() + 4.0),
        Vec2::new(bw, 18.0),
    );
    if theme::bevel_button(ui, row1, "选择 MIDI", Some(icons::FOLDER_OPEN), false, true).clicked()
    {
        actions.choose_midi = true;
    }
    let row1b = Rect::from_min_size(
        Pos2::new(row1.right() + gap, row1.top()),
        Vec2::new(bw, 18.0),
    );
    if theme::bevel_button(ui, row1b, "选择模板", Some(icons::ALBUM), false, true).clicked() {
        actions.choose_template = true;
    }
    let row2 = Rect::from_min_size(
        Pos2::new(rect.left() + 6.0, row1.bottom() + 3.0),
        Vec2::new(bw, 18.0),
    );
    if theme::bevel_button(ui, row2, "分析 MIDI", Some(icons::TUNE), false, true).clicked() {
        actions.analyze_midi = true;
    }
    let row2b = Rect::from_min_size(
        Pos2::new(row2.right() + gap, row2.top()),
        Vec2::new(bw, 18.0),
    );
    if theme::bevel_button(ui, row2b, "分析模板", Some(icons::INFO), false, true).clicked() {
        actions.analyze_template = true;
    }

    let status = if can_export {
        std::path::Path::new(app.output_path.trim())
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "输出 .als".to_string())
    } else {
        "选择 MIDI 后可导出".to_string()
    };
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(rect.left() + 8.0, row2b.bottom() + 2.0),
            Pos2::new(rect.right() - 6.0, rect.bottom() - 1.0),
        ),
        &status,
        8.8,
        theme::TEXT_DIM,
        Align2::LEFT_CENTER,
    );
}

fn draw_template_device(ui: &mut Ui, app: &mut App, rect: Rect, actions: &mut UiActions) {
    if rect.height() < 28.0 {
        return;
    }
    theme::shadowed_device(ui.painter(), rect);
    let header = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 18.0));
    ui.painter().rect_filled(
        header,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0xBE, 0xBE, 0xBE),
    );
    ui.painter().hline(
        header.x_range(),
        header.bottom() - 0.5,
        Stroke::new(1.0, theme::BORDER),
    );
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(header.left() + 8.0, header.top()),
            Pos2::new(header.right() - 6.0, header.bottom()),
        ),
        "模板",
        10.0,
        theme::TEXT,
        Align2::LEFT_CENTER,
    );
    let name = if app.template_path.trim().is_empty() {
        "内置 Ac Piano Upright"
    } else {
        std::path::Path::new(app.template_path.trim())
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("自定义模板")
    };
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(rect.left() + 8.0, header.bottom() + 1.0),
            Pos2::new(rect.right() - 6.0, header.bottom() + 20.0),
        ),
        name,
        9.0,
        theme::TEXT,
        Align2::LEFT_CENTER,
    );
    let button_y = header.bottom() + 22.0;
    if button_y + 18.0 <= rect.bottom() {
        let reset = Rect::from_min_size(
            Pos2::new(rect.left() + 6.0, button_y),
            Vec2::new((rect.width() - 16.0) * 0.5, 18.0),
        );
        if theme::bevel_button(ui, reset, "内置模板", None, false, true).clicked() {
            actions.clear_template = true;
        }
        let xml = Rect::from_min_size(
            Pos2::new(reset.right() + 4.0, button_y),
            Vec2::new((rect.width() - 16.0) * 0.5, 18.0),
        );
        if theme::bevel_button(ui, xml, "模板 XML", Some(icons::DOWNLOAD), false, true).clicked()
        {
            actions.dump_xml = true;
        }
    }
}

fn info_tab(ui: &mut Ui, app: &App, project: &ProjectView, rect: Rect) {
    let stats = app.midi_stats.as_ref();
    let result = app.last_result.as_ref();
    let template = app.template_report.as_ref();
    let n = |value: usize| value.to_string();
    let f = |value: f64| blacklive::format_num(value);
    let empty = "-".to_string();
    let midi_name = std::path::Path::new(app.midi_path.trim())
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| empty.clone());
    let output_name = std::path::Path::new(app.output_path.trim())
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| empty.clone());
    let template_name = if app.template_path.trim().is_empty() {
        "内置 Ac Piano Upright".to_string()
    } else {
        std::path::Path::new(app.template_path.trim())
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| empty.clone())
    };
    let rows: Vec<(&str, String, &str, String)> = vec![
        ("MIDI 文件", midi_name, "输出 .als", output_name),
        (
            "音符",
            stats
                .map(|s| n(s.note_count))
                .unwrap_or_else(|| empty.clone()),
            "PPQ",
            stats
                .map(|s| n(s.ppq as usize))
                .unwrap_or_else(|| empty.clone()),
        ),
        (
            "源轨",
            stats
                .map(|s| n(s.track_count))
                .unwrap_or_else(|| empty.clone()),
            "通道",
            stats
                .map(|s| n(s.notes_per_channel.len()))
                .unwrap_or_else(|| empty.clone()),
        ),
        (
            "最大复音",
            stats
                .map(|s| n(s.max_polyphony))
                .unwrap_or_else(|| empty.clone()),
            "时长",
            stats
                .map(|s| format!("{:.2} beats", s.duration_beats))
                .unwrap_or_else(|| empty.clone()),
        ),
        (
            "初始 BPM",
            stats
                .and_then(|s| s.initial_bpm)
                .map(f)
                .unwrap_or_else(|| empty.clone()),
            "拍号",
            stats
                .and_then(|s| s.time_signatures.first())
                .map(|sig| format!("{}/{}", sig.numerator, sig.denominator))
                .unwrap_or_else(|| "4/4".to_string()),
        ),
        (
            "Tempo 点",
            stats
                .map(|s| n(s.tempo_map.len()))
                .unwrap_or_else(|| empty.clone()),
            "RPN 事件",
            stats
                .map(|s| n(s.rpn_pitch_bend_range_count))
                .unwrap_or_else(|| empty.clone()),
        ),
        (
            "模板",
            template_name,
            "Live",
            template
                .map(|t| t.minor_version.clone())
                .unwrap_or_else(|| empty.clone()),
        ),
        (
            "模板轨道",
            template
                .map(|t| n(t.tracks.len()))
                .unwrap_or_else(|| empty.clone()),
            "模板 Clip",
            template
                .map(|t| n(t.clips.len()))
                .unwrap_or_else(|| empty.clone()),
        ),
        (
            "写入音符",
            result
                .map(|r| n(r.used_note_count))
                .unwrap_or_else(|| empty.clone()),
            "组 / 轨",
            result
                .map(|r| {
                    format!(
                        "{} / {}",
                        r.groups.len(),
                        r.groups.iter().map(|g| g.tracks.len()).sum::<usize>()
                    )
                })
                .unwrap_or_else(|| empty.clone()),
        ),
        (
            "丢弃（通道）",
            result
                .map(|r| n(r.dropped_by_channel))
                .unwrap_or_else(|| empty.clone()),
            "丢弃（移调）",
            result
                .map(|r| n(r.dropped_by_transpose))
                .unwrap_or_else(|| empty.clone()),
        ),
        (
            "缩短",
            result
                .map(|r| n(r.clamped_short_notes))
                .unwrap_or_else(|| empty.clone()),
            "同音高截短",
            result
                .map(|r| n(r.trimmed_overlaps))
                .unwrap_or_else(|| empty.clone()),
        ),
        (
            "Tempo 自动化",
            result
                .map(|r| if r.tempo_automation { "是" } else { "否" })
                .unwrap_or("否")
                .to_string(),
            "项目音符",
            n(project.total_notes),
        ),
    ];

    let mut table_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(Align::Min)),
    );
    table_ui.set_clip_rect(rect);
    table_ui.style_mut().spacing.item_spacing = Vec2::new(6.0, 1.0);
    TableBuilder::new(&mut table_ui)
        .id_salt("info_table")
        .resizable(false)
        .striped(true)
        .cell_layout(egui::Layout::left_to_right(Align::Center))
        .column(Column::exact(88.0))
        .column(Column::remainder())
        .column(Column::exact(88.0))
        .column(Column::remainder())
        .max_scroll_height(rect.height())
        .body(|mut body| {
            for (l1, v1, l2, v2) in rows {
                body.row(18.0, |mut row| {
                    row.col(|ui| {
                        theme::label_at(
                            ui.painter(),
                            ui.max_rect(),
                            l1,
                            9.5,
                            theme::TEXT_DIM,
                            Align2::LEFT_CENTER,
                        );
                    });
                    row.col(|ui| {
                        theme::label_at(
                            ui.painter(),
                            ui.max_rect().shrink2(Vec2::new(1.0, 0.0)),
                            &v1,
                            9.5,
                            theme::TEXT,
                            Align2::LEFT_CENTER,
                        );
                    });
                    row.col(|ui| {
                        theme::label_at(
                            ui.painter(),
                            ui.max_rect(),
                            l2,
                            9.5,
                            theme::TEXT_DIM,
                            Align2::LEFT_CENTER,
                        );
                    });
                    row.col(|ui| {
                        theme::label_at(
                            ui.painter(),
                            ui.max_rect().shrink2(Vec2::new(1.0, 0.0)),
                            &v2,
                            9.5,
                            theme::TEXT,
                            Align2::LEFT_CENTER,
                        );
                    });
                });
            }
        });
}

fn log_tab(ui: &mut Ui, app: &App, rect: Rect) {
    let painter = ui.painter().clone();
    painter.rect_filled(
        rect,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0x9B, 0x9B, 0x9B),
    );
    theme::frame_rect(&painter, rect, theme::BORDER_SOFT);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(6.0, 4.0)))
            .layout(egui::Layout::top_down(Align::Min)),
    );
    child.set_clip_rect(rect.shrink2(Vec2::new(6.0, 4.0)));
    egui::ScrollArea::vertical()
        .id_salt("detail_log")
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(&mut child, |ui| {
            if app.log.is_empty() {
                ui.label(
                    egui::RichText::new("还没有日志。选择 MIDI 或执行转换后，这里会显示结果。")
                        .size(9.5)
                        .color(theme::TEXT_DIM),
                );
                return;
            }
            for line in &app.log {
                let color = if line.starts_with('!') {
                    Color32::from_rgb(0xC4, 0x28, 0x28)
                } else if line.starts_with('⚠') {
                    Color32::from_rgb(0x8A, 0x5A, 0x00)
                } else {
                    theme::TEXT
                };
                ui.label(egui::RichText::new(line).size(9.5).color(color));
            }
        });
}
