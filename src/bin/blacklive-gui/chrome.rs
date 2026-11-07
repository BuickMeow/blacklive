use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2,
    ViewportCommand,
};

use super::App;
use super::actions::UiActions;
use super::icons;
use super::project::{ProjectView, position_label};
use super::theme;

pub(crate) fn title_bar(ui: &mut Ui, project: &ProjectView, _actions: &mut UiActions) {
    let rect = ui.max_rect();
    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::TITLE_BG);
    painter.hline(
        rect.x_range(),
        rect.top() + 0.5,
        Stroke::new(1.0, Color32::from_rgb(0x9E, 0x9E, 0x9E)),
    );
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, Color32::from_rgb(0x5E, 0x5E, 0x5E)),
    );

    // The left 78 px are reserved for the native macOS traffic lights.
    let drag_rect = Rect::from_min_max(Pos2::new(rect.left() + 78.0, rect.top()), rect.max);
    let drag = ui.interact(
        drag_rect,
        ui.make_persistent_id("title_drag"),
        Sense::click_and_drag(),
    );
    if drag.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    let focused = ui
        .ctx()
        .input(|input| input.viewport().focused.unwrap_or(true));
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(rect.left() + 100.0, rect.top()),
            Pos2::new(rect.right() - 100.0, rect.bottom()),
        ),
        &project.title,
        11.0,
        if focused {
            Color32::from_rgb(0x3B, 0x3B, 0x3B)
        } else {
            Color32::from_rgb(0x64, 0x64, 0x64)
        },
        Align2::CENTER_CENTER,
    );
}

pub(crate) fn transport_bar(
    ui: &mut Ui,
    app: &mut App,
    project: &ProjectView,
    actions: &mut UiActions,
) {
    let rect = ui.max_rect();
    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::TITLE_BG);
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, Color32::from_rgb(0x5E, 0x5E, 0x5E)),
    );

    let top = rect.top() + 1.0;
    let h = 16.0;
    let mut x = rect.left() + 8.0;

    let session_rect = Rect::from_min_size(Pos2::new(x, top), Vec2::new(15.0, h));
    if theme::icon_button(
        ui,
        session_rect,
        icons::VIEW_AGENDA,
        matches!(app.view_mode, super::project::ViewMode::Session),
        true,
    )
    .clicked()
    {
        actions.set_view = Some(super::project::ViewMode::Session);
    }
    x += 15.0;
    let arrangement_rect = Rect::from_min_size(Pos2::new(x, top), Vec2::new(15.0, h));
    if theme::icon_button(
        ui,
        arrangement_rect,
        icons::GRID_VIEW,
        matches!(app.view_mode, super::project::ViewMode::Arrangement),
        true,
    )
    .clicked()
    {
        actions.set_view = Some(super::project::ViewMode::Arrangement);
    }
    x += 15.0;
    let view_drop = Rect::from_min_size(Pos2::new(x, top), Vec2::new(12.0, h));
    if theme::icon_button(ui, view_drop, icons::ARROW_DROP_DOWN, false, true).clicked() {
        actions.set_view = Some(match app.view_mode {
            super::project::ViewMode::Session => super::project::ViewMode::Arrangement,
            super::project::ViewMode::Arrangement => super::project::ViewMode::Session,
        });
    }
    x += 18.0;

    let link_rect = Rect::from_min_size(Pos2::new(x, top), Vec2::new(42.0, h));
    if theme::bevel_button(ui, link_rect, "Link", None, app.link_enabled, true).clicked() {
        app.link_enabled = !app.link_enabled;
    }
    x += 46.0;
    let tap_rect = Rect::from_min_size(Pos2::new(x, top), Vec2::new(38.0, h));
    if theme::bevel_button(ui, tap_rect, "Tap", None, false, true).clicked() {
        app.tap_tempo();
    }
    x += 42.0;

    let bpm_rect = Rect::from_min_size(Pos2::new(x, top), Vec2::new(76.0, h));
    if let Some(buffer) = &mut app.bpm_edit {
        let response = theme::text_edit_at(ui, bpm_rect, buffer, "跟随 MIDI", 10.5, Align::Center);
        if app.bpm_focus {
            response.request_focus();
            app.bpm_focus = false;
        }
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        if enter || response.lost_focus() {
            app.commit_bpm_edit();
        }
    } else {
        let value = format!("{:.2}", project.bpm);
        theme::value_field(
            ui,
            bpm_rect,
            &value,
            Align2::CENTER_CENTER,
            10.5,
            theme::FIELD,
        );
        let response = ui.interact(bpm_rect, ui.make_persistent_id("bpm_field"), Sense::click());
        if response.clicked() {
            app.begin_bpm_edit();
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Text);
        }
    }
    x += 80.0;

    let metro_rect = Rect::from_min_size(Pos2::new(x, top), Vec2::new(24.0, h));
    if theme::bevel_button(ui, metro_rect, "", None, app.metronome_on, true).clicked() {
        app.metronome_on = !app.metronome_on;
    }
    paint_metronome(ui.painter(), metro_rect);
    x += 27.0;
    let sig = format!(
        "{} / {}",
        project.time_signature.0, project.time_signature.1
    );
    let sig_rect = Rect::from_min_size(Pos2::new(x, top), Vec2::new(54.0, h));
    theme::value_field(
        ui,
        sig_rect,
        &sig,
        Align2::CENTER_CENTER,
        10.5,
        theme::FIELD,
    );
    x += 58.0;
    let bar_rect = Rect::from_min_size(Pos2::new(x, top), Vec2::new(52.0, h));
    theme::bevel_button(ui, bar_rect, "1 Bar", None, false, true);
    paint_dropdown(
        ui.painter(),
        Pos2::new(bar_rect.right() - 9.0, bar_rect.center().y),
        1.0,
    );
    x += 56.0;

    if rect.width() > 1380.0 {
        let key_x = x.max(rect.left() + 350.0);
        let flat_rect = Rect::from_min_size(Pos2::new(key_x, top), Vec2::new(20.0, h));
        theme::bevel_button(ui, flat_rect, "♭", None, false, true);
        let key_rect = Rect::from_min_size(Pos2::new(key_x + 23.0, top), Vec2::new(30.0, h));
        theme::value_field(ui, key_rect, "C", Align2::CENTER_CENTER, 10.5, theme::FIELD);
        let mode_rect = Rect::from_min_size(Pos2::new(key_x + 56.0, top), Vec2::new(62.0, h));
        theme::bevel_button(
            ui,
            mode_rect,
            "Major",
            Some(icons::ARROW_DROP_DOWN),
            false,
            true,
        );
    }

    // Centered transport cluster.
    let cluster_w = if rect.width() > 1480.0 { 430.0 } else { 350.0 };
    let mut tx = (rect.center().x - cluster_w * 0.5).max(x + 16.0);

    let capture_rect = Rect::from_min_size(Pos2::new(tx, top), Vec2::new(24.0, h));
    if theme::icon_button(ui, capture_rect, icons::ARROW_RIGHT, true, true).clicked() {
        actions.choose_midi = true;
    }
    tx += 27.0;
    let pos1 = position_label(app.playhead_beats, project.time_signature);
    let pos1_rect = Rect::from_min_size(Pos2::new(tx, top), Vec2::new(90.0, h));
    theme::value_field(
        ui,
        pos1_rect,
        &pos1,
        Align2::CENTER_CENTER,
        10.5,
        theme::FIELD,
    );
    tx += 94.0;

    let play_rect = Rect::from_min_size(Pos2::new(tx, top), Vec2::new(34.0, h));
    let play_response = theme::bevel_button(ui, play_rect, "", None, false, true);
    let play_color = if app.playing {
        theme::ACCENT_DARK
    } else {
        theme::TEXT
    };
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(play_rect.center().x - 5.0, play_rect.center().y - 7.0),
            Pos2::new(play_rect.center().x - 5.0, play_rect.center().y + 7.0),
            Pos2::new(play_rect.center().x + 8.0, play_rect.center().y),
        ],
        play_color,
        Stroke::NONE,
    ));
    if play_response.clicked() {
        app.toggle_play();
    }
    tx += 36.0;

    let stop_rect = Rect::from_min_size(Pos2::new(tx, top), Vec2::new(30.0, h));
    let stop_response = theme::bevel_button(ui, stop_rect, "", None, false, true);
    ui.painter().rect_filled(
        Rect::from_center_size(stop_rect.center(), Vec2::splat(14.0)),
        egui::CornerRadius::ZERO,
        theme::TEXT,
    );
    if stop_response.clicked() {
        app.stop();
    }
    tx += 32.0;

    let record_rect = Rect::from_min_size(Pos2::new(tx, top), Vec2::new(30.0, h));
    let record_response = theme::bevel_button(ui, record_rect, "", None, false, true);
    ui.painter()
        .circle_filled(record_rect.center(), 7.0, theme::TEXT);
    if record_response.clicked() {
        app.toggle_record_arm();
    }
    tx += 36.0;

    let plus_rect = Rect::from_min_size(Pos2::new(tx, top), Vec2::new(20.0, h));
    theme::bevel_button(ui, plus_rect, "", Some(icons::ADD), false, true);
    tx += 23.0;
    let link2 = Rect::from_min_size(Pos2::new(tx, top), Vec2::new(24.0, h));
    if theme::icon_button(ui, link2, icons::LINK, app.link_enabled, true).clicked() {
        app.link_enabled = !app.link_enabled;
    }
    tx += 27.0;
    let loop_rect = Rect::from_min_size(Pos2::new(tx, top), Vec2::new(24.0, h));
    if theme::icon_button(ui, loop_rect, icons::LOOP, app.loop_enabled, true).clicked() {
        app.loop_enabled = !app.loop_enabled;
    }
    tx += 28.0;

    if rect.width() > 1480.0 {
        let pos2 = position_label(project.duration_beats.max(0.0), project.time_signature);
        let pos2_rect = Rect::from_min_size(Pos2::new(tx, top), Vec2::new(88.0, h));
        theme::value_field(
            ui,
            pos2_rect,
            &pos2,
            Align2::CENTER_CENTER,
            10.5,
            theme::FIELD,
        );
        tx += 92.0;
        let waves_rect = Rect::from_min_size(Pos2::new(tx, top), Vec2::new(24.0, h));
        theme::icon_button(ui, waves_rect, icons::WAVES, false, true);
        tx += 26.0;
        let tune_rect = Rect::from_min_size(Pos2::new(tx, top), Vec2::new(24.0, h));
        theme::icon_button(ui, tune_rect, icons::TUNE, false, true);
    }

    // Right-side status and primary actions.
    let mut rx = rect.right() - 8.0;
    let export_rect = Rect::from_min_size(Pos2::new(rx - 96.0, top), Vec2::new(96.0, h));
    rx -= 100.0;
    let choose_rect = Rect::from_min_size(Pos2::new(rx - 96.0, top), Vec2::new(96.0, h));
    rx -= 104.0;

    let can_export = !app.midi_path.trim().is_empty();
    if theme::bevel_button(
        ui,
        export_rect,
        "导出 .als",
        Some(icons::SAVE),
        can_export,
        can_export,
    )
    .clicked()
    {
        actions.convert = true;
    }
    if theme::bevel_button(
        ui,
        choose_rect,
        "选择 MIDI",
        Some(icons::FOLDER_OPEN),
        false,
        true,
    )
    .clicked()
    {
        actions.choose_midi = true;
    }

    if rect.width() > 1270.0 {
        let midi_rect = Rect::from_min_size(Pos2::new(rx - 44.0, top), Vec2::new(40.0, h));
        theme::mixed_icon_label(
            ui.painter(),
            midi_rect,
            icons::PIANO,
            "MIDI",
            9.5,
            theme::TEXT,
        );
        rx -= 48.0;
        let sr_rect = Rect::from_min_size(Pos2::new(rx - 66.0, top), Vec2::new(64.0, h));
        theme::label_at(
            ui.painter(),
            sr_rect,
            "48.0 kHz",
            9.5,
            theme::TEXT,
            Align2::RIGHT_CENTER,
        );
        rx -= 70.0;
        let cpu_rect = Rect::from_min_size(Pos2::new(rx - 44.0, top), Vec2::new(42.0, h));
        theme::label_at(
            ui.painter(),
            cpu_rect,
            "0 %",
            9.5,
            theme::TEXT,
            Align2::RIGHT_CENTER,
        );
        rx -= 46.0;
        let meter_rect =
            Rect::from_min_size(Pos2::new(rx - 44.0, top + 4.0), Vec2::new(42.0, h - 8.0));
        paint_meter(ui.painter(), meter_rect, 0.34);
        rx -= 50.0;
    }

    if app.view_mode == super::project::ViewMode::Arrangement {
        let status = if app.playing {
            format!(
                "{}  ·  播放中",
                position_label(app.playhead_beats, project.time_signature)
            )
        } else {
            String::new()
        };
        if !status.is_empty() {
            theme::label_at(
                ui.painter(),
                Rect::from_min_size(Pos2::new(rx - 150.0, top), Vec2::new(146.0, h)),
                &status,
                9.5,
                theme::TEXT_DIM,
                Align2::RIGHT_CENTER,
            );
        }
    }
}

fn paint_metronome(painter: &egui::Painter, rect: Rect) {
    let c = rect.center();
    let body = [
        Pos2::new(c.x - 5.5, c.y + 6.0),
        Pos2::new(c.x - 2.5, c.y - 6.0),
        Pos2::new(c.x + 2.5, c.y - 6.0),
        Pos2::new(c.x + 5.5, c.y + 6.0),
    ];
    for pair in [[0, 1], [1, 2], [2, 3], [3, 0]] {
        painter.line_segment(
            [body[pair[0]], body[pair[1]]],
            Stroke::new(1.0, theme::TEXT),
        );
    }
    painter.line_segment(
        [Pos2::new(c.x, c.y + 4.0), Pos2::new(c.x + 3.0, c.y - 4.0)],
        Stroke::new(1.2, theme::TEXT),
    );
    painter.circle_filled(Pos2::new(c.x + 3.0, c.y - 4.0), 1.5, theme::TEXT);
}

fn paint_dropdown(painter: &egui::Painter, center: Pos2, scale: f32) {
    let w = 3.5 * scale;
    let h = 1.8 * scale;
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(center.x - w, center.y - h),
            Pos2::new(center.x + w, center.y - h),
            Pos2::new(center.x, center.y + h + 0.8),
        ],
        theme::TEXT,
        Stroke::NONE,
    ));
}

fn paint_meter(painter: &egui::Painter, rect: Rect, value: f32) {
    painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::METER_BG);
    let inner = rect.shrink(2.0);
    let fill_w = inner.width() * value.clamp(0.0, 1.0);
    painter.rect_filled(
        Rect::from_min_size(inner.min, Vec2::new(fill_w, inner.height())),
        egui::CornerRadius::ZERO,
        theme::METER_GREEN,
    );
    theme::frame_rect(painter, rect, theme::BORDER);
}

pub(crate) fn status_bar(ui: &mut Ui, app: &App) {
    let rect = ui.max_rect();
    let painter = ui.painter().clone();
    let top_strip = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 8.0));
    let info = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + 8.0),
        Pos2::new(rect.right(), rect.bottom() - 8.0),
    );
    let bottom_strip = Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - 8.0), rect.max);
    painter.rect_filled(top_strip, egui::CornerRadius::ZERO, theme::TITLE_BG);
    painter.rect_filled(info, egui::CornerRadius::ZERO, theme::SURFACE);
    painter.rect_filled(bottom_strip, egui::CornerRadius::ZERO, theme::TITLE_BG);
    painter.hline(
        top_strip.x_range(),
        top_strip.bottom() - 0.5,
        Stroke::new(1.0, theme::BORDER_SOFT),
    );
    painter.hline(
        info.x_range(),
        info.top() + 0.5,
        Stroke::new(1.0, theme::PANEL_HI),
    );
    painter.hline(
        info.x_range(),
        info.bottom() - 0.5,
        Stroke::new(1.0, theme::BORDER_SOFT),
    );

    // A minimal progress/info strip, mirroring Live's bottom bar.
    let progress = Rect::from_min_size(
        Pos2::new(rect.left() + 34.0, top_strip.top() + 2.0),
        Vec2::new((rect.width() * 0.36).min(420.0), 3.0),
    );
    painter.rect_filled(
        progress,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0x6C, 0x6C, 0x6C),
    );
    let handle = Rect::from_min_size(
        Pos2::new(progress.left() + 30.0, progress.top()),
        Vec2::new(progress.width() * 0.42, progress.height()),
    );
    painter.rect_filled(
        handle,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0xB8, 0xB8, 0xB8),
    );

    let color = if app.status_is_error {
        Color32::from_rgb(0xC4, 0x28, 0x28)
    } else {
        theme::TEXT
    };
    painter.text(
        Pos2::new(info.left() + 10.0, info.center().y),
        Align2::LEFT_CENTER,
        icons::INFO,
        icons::font(12.0),
        color,
    );
    theme::label_at(
        &painter,
        Rect::from_min_max(
            Pos2::new(info.left() + 28.0, info.top()),
            Pos2::new(info.right() - 330.0, info.bottom()),
        ),
        &app.status,
        10.0,
        color,
        Align2::LEFT_CENTER,
    );

    let summary = if let Some(result) = &app.last_result {
        format!(
            "{} 音符 · {} 组 · BPM {}",
            result.used_note_count,
            result.groups.len(),
            blacklive::format_num(result.bpm)
        )
    } else {
        "Blacklive 0.1".to_string()
    };
    theme::label_at(
        &painter,
        Rect::from_min_max(
            Pos2::new(info.right() - 326.0, info.top()),
            Pos2::new(info.right() - 72.0, info.bottom()),
        ),
        &summary,
        9.5,
        theme::TEXT_DIM,
        Align2::RIGHT_CENTER,
    );
    let midi_badge = Rect::from_min_size(
        Pos2::new(info.right() - 66.0, info.top() + 2.0),
        Vec2::new(58.0, info.height() - 4.0),
    );
    theme::value_field(
        ui,
        midi_badge,
        if app.midi_path.trim().is_empty() {
            "MIDI"
        } else {
            "A01"
        },
        Align2::CENTER_CENTER,
        8.5,
        Color32::from_rgb(0x9B, 0xC9, 0xE8),
    );
    theme::label_at(
        &painter,
        Rect::from_min_max(
            Pos2::new(info.right() - 160.0, top_strip.top()),
            Pos2::new(info.right() - 8.0, top_strip.bottom()),
        ),
        "48.0 kHz",
        8.0,
        theme::TEXT_DIM,
        Align2::RIGHT_CENTER,
    );
    painter.text(
        Pos2::new(rect.left() + 6.0, info.center().y),
        Align2::LEFT_CENTER,
        "●",
        FontId::proportional(7.0),
        Color32::from_rgb(0x6E, 0xA8, 0x6E),
    );
}
