use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2,
};

use super::App;
use super::actions::UiActions;
use super::icons;
use super::project::{ProjectView, TrackKind};
use super::theme;

const HEADER_H: f32 = 62.0;
const STRIP_H: f32 = 8.0;
const RULER_H: f32 = 30.0;
const BAR_W: f32 = 16.0;
const SCROLL_H: f32 = 14.0;
const TIME_RULER_H: f32 = 16.0;
const RETURN_H: f32 = 17.0;

pub(crate) fn show(ui: &mut Ui, app: &mut App, project: &ProjectView, actions: &mut UiActions) {
    let rect = ui.max_rect();
    let header_w = theme::HEADER_W;
    let timeline_w = (rect.width() - header_w).max(240.0);
    let return_area_h = RETURN_H * 3.0 + 6.0;
    let returns_bottom = rect.bottom() - SCROLL_H - TIME_RULER_H;
    let tracks_bottom = (returns_bottom - return_area_h).max(rect.top() + HEADER_H);
    let track_area = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + HEADER_H),
        Pos2::new(rect.left() + timeline_w, tracks_bottom),
    );
    let header_area = Rect::from_min_max(
        Pos2::new(rect.left() + timeline_w, rect.top() + HEADER_H),
        Pos2::new(rect.right(), rect.bottom()),
    );

    let painter = ui.painter().clone();
    painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE_DARK);
    let _ = actions;

    // Scroll interactions.
    if let Some(pos) = ui.input(|input| input.pointer.hover_pos())
        && rect.contains(pos)
    {
        let (scroll, modifiers) = ui.input(|input| (input.smooth_scroll_delta, input.modifiers));
        if modifiers.shift || scroll.x.abs() > scroll.y.abs() {
            app.h_offset_bars = (app.h_offset_bars - scroll.x as f64 / BAR_W as f64).max(0.0);
        } else {
            app.arr_scroll_y -= scroll.y;
        }
    }
    let visible_tracks_h = (tracks_bottom - (rect.top() + HEADER_H)).max(0.0);
    let total_tracks_h = project.tracks.len() as f32 * theme::TRACK_H;
    let max_scroll = (total_tracks_h - visible_tracks_h).max(0.0);
    app.arr_scroll_y = app.arr_scroll_y.clamp(0.0, max_scroll);

    draw_grid(&painter, rect, track_area, timeline_w);
    draw_ruler(ui, rect, timeline_w, app, project);
    draw_tracks(ui, app, project, rect, track_area, timeline_w);
    draw_headers(ui, app, project, header_area, rect.top());
    draw_vscroll(
        ui,
        app,
        project,
        rect,
        timeline_w,
        tracks_bottom,
        max_scroll,
    );
    draw_returns(
        ui,
        app,
        project,
        rect,
        timeline_w,
        tracks_bottom,
        returns_bottom,
    );
    draw_time_ruler(ui, project, app, rect, timeline_w, returns_bottom);
    draw_scrollbar(ui, app, project, rect, timeline_w, tracks_bottom);
    draw_playhead(ui, app, project, timeline_w, rect.top());
    if project.placeholder && !project.tracks.iter().any(|track| !track.clips.is_empty()) {
        let hint = Rect::from_min_max(
            Pos2::new(rect.left(), rect.top() + HEADER_H),
            Pos2::new(rect.left() + timeline_w, tracks_bottom),
        );
        theme::label_at(
            &painter,
            hint,
            "在这里加载文件和装置",
            10.0,
            Color32::from_rgb(0x6B, 0x6B, 0x6B),
            Align2::CENTER_CENTER,
        );
    }

    // Header column finish.
    painter.vline(
        rect.left() + timeline_w - 0.5,
        rect.y_range(),
        Stroke::new(1.0, theme::BORDER),
    );
    painter.vline(
        rect.left() + timeline_w + 0.5,
        rect.y_range(),
        Stroke::new(1.0, Color32::from_rgb(0xC0, 0xC0, 0xC0)),
    );
}

fn draw_grid(painter: &egui::Painter, rect: Rect, track_area: Rect, timeline_w: f32) {
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(rect.left(), rect.top() + HEADER_H),
            Pos2::new(rect.left() + timeline_w, rect.bottom() - SCROLL_H),
        ),
        egui::CornerRadius::ZERO,
        theme::SURFACE_DARK,
    );
    let p = painter.with_clip_rect(Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + HEADER_H),
        Pos2::new(rect.left() + timeline_w, rect.bottom() - SCROLL_H),
    ));
    let start_bar = 0.0f64;
    let mut x = rect.left();
    let mut bar = start_bar;
    while x < rect.left() + timeline_w + BAR_W {
        let strong = (bar.round() as i64) % 4 == 0;
        p.vline(
            x.round() + 0.5,
            (rect.top() + HEADER_H)..=(rect.bottom() - SCROLL_H),
            Stroke::new(
                1.0,
                if strong {
                    Color32::from_rgb(0x8B, 0x8B, 0x8B)
                } else {
                    Color32::from_rgb(0xA0, 0xA0, 0xA0)
                },
            ),
        );
        x += BAR_W;
        bar += 1.0;
    }
    let _ = track_area;
}

fn draw_ruler(ui: &mut Ui, rect: Rect, timeline_w: f32, app: &App, project: &ProjectView) {
    let painter = ui.painter().clone();
    let _ = project;
    painter.rect_filled(
        Rect::from_min_size(
            Pos2::new(rect.left() + timeline_w, rect.top()),
            Vec2::new(rect.width() - timeline_w, HEADER_H),
        ),
        egui::CornerRadius::ZERO,
        theme::SURFACE,
    );
    painter.rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(timeline_w, HEADER_H)),
        egui::CornerRadius::ZERO,
        theme::SURFACE,
    );
    // Diagonal striped strip at the top.
    let strip = Rect::from_min_size(rect.min, Vec2::new(timeline_w, STRIP_H));
    painter.rect_filled(
        strip,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0xB8, 0xB8, 0xB8),
    );
    let p = painter.with_clip_rect(strip);
    let mut x = strip.left() - STRIP_H;
    while x < strip.right() {
        p.line_segment(
            [
                Pos2::new(x, strip.bottom()),
                Pos2::new(x + STRIP_H, strip.top()),
            ],
            Stroke::new(1.0, Color32::from_rgb(0x86, 0x86, 0x86)),
        );
        x += 6.0;
    }
    painter.hline(
        strip.x_range(),
        strip.bottom() - 0.5,
        Stroke::new(1.0, theme::BORDER_SOFT),
    );

    // Loop bracket.
    if app.loop_enabled {
        let y = rect.top() + STRIP_H + 14.0;
        let start_x = rect.left() + 18.0;
        let end_x = start_x + 4.0 * BAR_W;
        painter.hline(
            start_x..=end_x,
            y,
            Stroke::new(1.0, Color32::from_rgb(0x33, 0x33, 0x33)),
        );
        painter.vline(
            start_x,
            y..=y + 15.0,
            Stroke::new(1.0, Color32::from_rgb(0x33, 0x33, 0x33)),
        );
        painter.vline(
            end_x,
            y..=y + 15.0,
            Stroke::new(1.0, Color32::from_rgb(0x33, 0x33, 0x33)),
        );
        for (x, dir) in [(start_x, 1.0), (end_x, -1.0)] {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(x, y + 3.0),
                    Pos2::new(x + 6.0 * dir, y + 7.0),
                    Pos2::new(x, y + 11.0),
                ],
                Color32::from_rgb(0x33, 0x33, 0x33),
                Stroke::NONE,
            ));
        }
    }

    // Bar numbers, every four bars.
    let ruler_top = rect.top() + HEADER_H - RULER_H;
    painter.hline(
        rect.left()..=(rect.left() + timeline_w),
        ruler_top + 0.5,
        Stroke::new(1.0, theme::BORDER_SOFT),
    );
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(rect.left(), ruler_top + 1.0),
            Pos2::new(rect.left() + timeline_w, rect.top() + HEADER_H),
        ),
        egui::CornerRadius::ZERO,
        theme::SURFACE,
    );
    let p = painter.with_clip_rect(Rect::from_min_max(
        Pos2::new(rect.left(), ruler_top),
        Pos2::new(rect.left() + timeline_w, rect.top() + HEADER_H),
    ));
    let visible_bars = (timeline_w / BAR_W).ceil() as i64 + 2;
    let mut bar = 1i64;
    while bar <= visible_bars {
        let x = rect.left() + ((bar - 1) as f64 - app.h_offset_bars) as f32 * BAR_W + 18.0;
        if x >= rect.left() - 20.0 && x <= rect.left() + timeline_w {
            if (bar - 1) % 4 == 0 {
                p.vline(
                    x.round(),
                    (ruler_top + 12.0)..=(rect.top() + HEADER_H),
                    Stroke::new(1.0, Color32::from_rgb(0x72, 0x72, 0x72)),
                );
                p.text(
                    Pos2::new(x + 2.0, ruler_top + 8.0),
                    Align2::LEFT_CENTER,
                    bar.to_string(),
                    FontId::proportional(9.5),
                    Color32::from_rgb(0x3D, 0x3D, 0x3D),
                );
            } else {
                p.vline(
                    x.round() + 0.5,
                    (rect.top() + HEADER_H - 8.0)..=(rect.top() + HEADER_H),
                    Stroke::new(1.0, Color32::from_rgb(0x84, 0x84, 0x84)),
                );
            }
        }
        bar += 1;
    }

    // Top-right Set / follow / lock controls.
    let set_x = rect.left() + timeline_w + 8.0;
    let set_rect = Rect::from_min_size(
        Pos2::new(set_x, rect.top() + STRIP_H + 5.0),
        Vec2::new(38.0, 16.0),
    );
    theme::bevel_button(ui, set_rect, "Set", None, false, true);
    let folder_rect = Rect::from_min_size(
        Pos2::new(set_rect.right() + 4.0, set_rect.top() + 1.0),
        Vec2::new(16.0, 14.0),
    );
    theme::icon_button(ui, folder_rect, icons::FOLDER_OPEN, false, true);
    let lock_rect = Rect::from_min_size(
        Pos2::new(folder_rect.right() + 4.0, set_rect.top() + 1.0),
        Vec2::new(16.0, 14.0),
    );
    theme::icon_button(ui, lock_rect, icons::LOCK, false, true);
    let left_chev = Rect::from_min_size(
        Pos2::new(set_x + 20.0, set_rect.bottom() + 4.0),
        Vec2::new(16.0, 14.0),
    );
    theme::icon_button(ui, left_chev, icons::CHEVRON_LEFT, false, true);
    let right_chev = Rect::from_min_size(
        Pos2::new(left_chev.right() + 4.0, left_chev.top()),
        Vec2::new(16.0, 14.0),
    );
    theme::icon_button(ui, right_chev, icons::CHEVRON_RIGHT, false, true);
}

fn draw_tracks(
    ui: &mut Ui,
    app: &mut App,
    project: &ProjectView,
    rect: Rect,
    track_area: Rect,
    timeline_w: f32,
) {
    let p = ui.painter().with_clip_rect(Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + HEADER_H),
        Pos2::new(rect.left() + timeline_w, rect.bottom() - SCROLL_H),
    ));
    let top = rect.top() + HEADER_H - app.arr_scroll_y;
    for (index, track) in project.tracks.iter().enumerate() {
        let y = top + index as f32 * theme::TRACK_H;
        if y + theme::TRACK_H < rect.top() + HEADER_H || y > rect.bottom() - SCROLL_H {
            continue;
        }
        let lane = Rect::from_min_size(
            Pos2::new(rect.left(), y),
            Vec2::new(timeline_w, theme::TRACK_H),
        );
        let fill = if index % 2 == 0 {
            Color32::from_rgb(0x9B, 0x9B, 0x9B)
        } else {
            Color32::from_rgb(0xB0, 0xB0, 0xB0)
        };
        p.rect_filled(lane, egui::CornerRadius::ZERO, fill);
        p.hline(
            lane.x_range(),
            lane.bottom() - 0.5,
            Stroke::new(1.0, Color32::from_rgb(0x86, 0x86, 0x86)),
        );

        for clip in &track.clips {
            let start = clip.start_beats / 4.0 - app.h_offset_bars;
            let length = (clip.length_beats / 4.0).max(0.25);
            let x = lane.left() + 18.0 + start as f32 * BAR_W;
            let w = (length as f32 * BAR_W).max(18.0);
            let clip_rect = Rect::from_min_max(
                Pos2::new(x, lane.top() + 4.0),
                Pos2::new(x + w, lane.bottom() - 4.0),
            );
            if clip_rect.right() < lane.left() || clip_rect.left() > lane.right() {
                continue;
            }
            draw_clip(&p, clip_rect, clip, &track.name);
        }

        // Empty-lane click selection.
        let id = ui.make_persistent_id(("arr_lane", index));
        let response = ui.interact(lane, id, Sense::click());
        if response.clicked() {
            app.selected_track = Some(index);
        }
    }
    let _ = track_area;
}

fn draw_clip(
    painter: &egui::Painter,
    rect: Rect,
    clip: &super::project::ClipView,
    track_name: &str,
) {
    let p = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    let fill = clip.color;
    p.rect_filled(rect, egui::CornerRadius::ZERO, fill);
    let header = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 13.0));
    p.rect_filled(
        header,
        egui::CornerRadius::ZERO,
        theme::mix(fill, Color32::BLACK, 0.18),
    );
    p.hline(
        header.x_range(),
        header.bottom() - 0.5,
        Stroke::new(1.0, theme::mix(fill, Color32::BLACK, 0.4)),
    );
    theme::label_at(
        &p,
        header.shrink2(Vec2::new(3.0, 0.0)),
        &clip.name,
        9.0,
        Color32::from_rgb(0x1A, 0x1A, 0x1A),
        Align2::LEFT_CENTER,
    );
    // A restrained waveform suggestion, generated from the track name so it is stable.
    let inner = rect.shrink2(Vec2::new(3.0, 19.0));
    if inner.height() > 8.0 {
        let seed = track_name.bytes().fold(0u32, |acc, byte| {
            acc.wrapping_mul(31).wrapping_add(byte as u32)
        });
        let wave_color = theme::mix(fill, Color32::BLACK, 0.22);
        let mut x = inner.left() + 2.0;
        let mut i = 0u32;
        while x < inner.right() - 2.0 {
            let value =
                ((seed.wrapping_add(i.wrapping_mul(2654435761)) >> 9) & 0xFF) as f32 / 255.0;
            let h = 3.0 + value * (inner.height() - 3.0);
            p.vline(
                x,
                (inner.bottom() - h)..=inner.bottom(),
                Stroke::new(1.0, wave_color),
            );
            x += 4.0;
            i += 1;
        }
    }
    theme::frame_rect(painter, rect, theme::mix(fill, Color32::BLACK, 0.55));
}

fn draw_headers(ui: &mut Ui, app: &mut App, project: &ProjectView, header_area: Rect, top: f32) {
    let painter = ui.painter().with_clip_rect(header_area);
    let painter = &painter;
    painter.rect_filled(header_area, egui::CornerRadius::ZERO, theme::SURFACE);
    let header_top = top + HEADER_H;
    let starts = header_top - app.arr_scroll_y;
    let color_w = 94.0;
    let io_w = 66.0;

    for (index, track) in project.tracks.iter().enumerate() {
        let y = starts + index as f32 * theme::TRACK_H;
        if y < header_top - 0.5 || y + theme::TRACK_H > header_area.bottom() + 0.5 {
            continue;
        }
        let row = Rect::from_min_size(
            Pos2::new(header_area.left(), y),
            Vec2::new(header_area.width(), theme::TRACK_H),
        );
        let selected = app.selected_track == Some(index);
        painter.rect_filled(row, egui::CornerRadius::ZERO, theme::SURFACE);
        painter.hline(
            row.x_range(),
            row.top() + 0.5,
            Stroke::new(1.0, theme::PANEL_HI),
        );
        painter.hline(
            row.x_range(),
            row.bottom() - 0.5,
            Stroke::new(1.0, theme::BORDER_SOFT),
        );

        let color_rect = Rect::from_min_size(row.min, Vec2::new(color_w, row.height()));
        let header_fill = if selected {
            theme::mix(track.color, Color32::WHITE, 0.06)
        } else {
            track.color
        };
        painter.rect_filled(color_rect, egui::CornerRadius::ZERO, header_fill);
        painter.vline(
            color_rect.right() - 0.5,
            color_rect.y_range(),
            Stroke::new(1.0, theme::mix(header_fill, Color32::BLACK, 0.3)),
        );

        let circle_center = Pos2::new(color_rect.left() + 12.0, color_rect.top() + 16.0);
        painter.circle_stroke(
            circle_center,
            7.0,
            Stroke::new(1.2, Color32::from_rgb(0x17, 0x17, 0x17)),
        );
        painter.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(circle_center.x - 3.5, circle_center.y - 2.0),
                Pos2::new(circle_center.x + 3.5, circle_center.y - 2.0),
                Pos2::new(circle_center.x, circle_center.y + 3.5),
            ],
            theme::TEXT,
            Stroke::NONE,
        ));
        theme::label_at(
            painter,
            Rect::from_min_max(
                Pos2::new(color_rect.left() + 23.0, color_rect.top() + 7.0),
                Pos2::new(color_rect.right() - 4.0, color_rect.top() + 27.0),
            ),
            &track.name,
            11.0,
            theme::TEXT,
            Align2::LEFT_CENTER,
        );

        let io_x = color_rect.right() + 4.0;
        let io_rect = Rect::from_min_size(
            Pos2::new(io_x, row.top() + 2.0),
            Vec2::new(io_w, row.height() - 4.0),
        );
        draw_io_fields(painter, io_rect, track.kind);

        let right_x = io_rect.right() + 4.0;
        let right_rect = Rect::from_min_max(
            Pos2::new(right_x, row.top() + 2.0),
            Pos2::new(row.right() - 6.0, row.bottom() - 2.0),
        );
        painter.vline(
            right_rect.left() - 2.0,
            row.y_range(),
            Stroke::new(1.0, theme::BORDER_SOFT),
        );
        let number_rect = Rect::from_min_size(
            Pos2::new(right_rect.left(), right_rect.top()),
            Vec2::new(60.0, 17.0),
        );
        theme::value_field(
            ui,
            number_rect,
            &(index + 1).to_string(),
            Align2::CENTER_CENTER,
            10.5,
            theme::ACCENT_FIELD,
        );
        let solo_rect = Rect::from_min_size(
            Pos2::new(number_rect.right() + 4.0, number_rect.top()),
            Vec2::new(20.0, 17.0),
        );
        if theme::bevel_button(
            ui,
            solo_rect,
            "S",
            None,
            app.solo_tracks.contains(&index),
            true,
        )
        .clicked()
            && !app.solo_tracks.insert(index)
        {
            app.solo_tracks.remove(&index);
        }
        let arm_rect = Rect::from_min_size(
            Pos2::new(solo_rect.right() + 4.0, number_rect.top()),
            Vec2::new(20.0, 17.0),
        );
        if theme::icon_button(
            ui,
            arm_rect,
            icons::FIBER_MANUAL_RECORD,
            app.armed_tracks.contains(&index),
            true,
        )
        .clicked()
            && !app.armed_tracks.insert(index)
        {
            app.armed_tracks.remove(&index);
        }

        // Meter dots down the right edge.
        let meter_x = right_rect.right() - 5.0;
        for dot in 0..14 {
            painter.circle_filled(
                Pos2::new(meter_x, right_rect.top() + 12.0 + dot as f32 * 5.2),
                1.7,
                Color32::from_rgb(0x1F, 0x1F, 0x1F),
            );
        }

        // Header click / selection.
        let header_click = Rect::from_min_max(
            Pos2::new(row.left(), row.top()),
            Pos2::new(io_x + io_w + 4.0, row.bottom()),
        );
        let response = ui.interact(
            header_click,
            ui.make_persistent_id(("track_header", index)),
            Sense::click(),
        );
        if response.clicked() {
            app.selected_track = Some(index);
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
    }
}

fn draw_io_fields(painter: &egui::Painter, rect: Rect, kind: TrackKind) {
    let values = match kind {
        TrackKind::Midi => ["All Ins", "All Chann", "In  Auto  Off", "No Output"],
        TrackKind::Audio => ["Ext. In", "1", "In  Auto  Off", "Main"],
    };
    for (index, text) in values.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(rect.left(), rect.top() + index as f32 * 19.0),
            Vec2::new(rect.width(), 17.0),
        );
        if index == 2 && *text == "In  Auto  Off" {
            let segments = ["In", "Auto", "Off"];
            let widths = [18.0, 28.0, 18.0];
            let mut x = r.left();
            for (segment, width) in segments.iter().zip(widths) {
                let seg = Rect::from_min_size(Pos2::new(x, r.top()), Vec2::new(width, r.height()));
                painter.rect_filled(
                    seg,
                    egui::CornerRadius::ZERO,
                    if *segment == "Auto" {
                        theme::ACCENT_FIELD
                    } else {
                        Color32::from_rgb(0xC9, 0xC9, 0xC9)
                    },
                );
                theme::frame_rect(painter, seg, theme::BORDER_SOFT);
                painter.text(
                    seg.center(),
                    Align2::CENTER_CENTER,
                    *segment,
                    FontId::proportional(9.0),
                    theme::TEXT,
                );
                x += width;
            }
        } else {
            painter.rect_filled(
                r,
                egui::CornerRadius::ZERO,
                Color32::from_rgb(0xE1, 0xE1, 0xE1),
            );
            theme::frame_rect(painter, r, theme::BORDER_SOFT);
            painter.text(
                Pos2::new(r.left() + 5.0, r.center().y),
                Align2::LEFT_CENTER,
                *text,
                FontId::proportional(8.6),
                theme::TEXT,
            );
            let cx = r.right() - 8.0;
            let cy = r.center().y;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(cx - 3.5, cy - 1.8),
                    Pos2::new(cx + 3.5, cy - 1.8),
                    Pos2::new(cx, cy + 2.4),
                ],
                theme::TEXT,
                Stroke::NONE,
            ));
        }
    }
}

fn draw_vscroll(
    ui: &mut Ui,
    app: &mut App,
    _project: &ProjectView,
    rect: Rect,
    timeline_w: f32,
    tracks_bottom: f32,
    max_scroll: f32,
) {
    if max_scroll <= 0.5 {
        return;
    }
    let painter = ui.painter().clone();
    let area = Rect::from_min_max(
        Pos2::new(rect.right() - 6.0, rect.top() + HEADER_H),
        Pos2::new(rect.right(), tracks_bottom),
    );
    painter.rect_filled(
        area,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0x8B, 0x8B, 0x8B),
    );
    let visible = (tracks_bottom - rect.top() - HEADER_H).max(1.0);
    let total = visible + max_scroll;
    let handle_h = (visible / total * area.height()).max(28.0);
    let frac = (app.arr_scroll_y / max_scroll).clamp(0.0, 1.0);
    let handle = Rect::from_min_size(
        Pos2::new(area.left(), area.top() + frac * (area.height() - handle_h)),
        Vec2::new(area.width(), handle_h),
    );
    painter.rect_filled(
        handle,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0x6D, 0x6D, 0x6D),
    );
    theme::frame_rect(&painter, handle, theme::BORDER_SOFT);
    let response = ui.interact(handle, ui.make_persistent_id("arr_vscroll"), Sense::drag());
    if response.dragged() {
        let delta = response.drag_delta().y / (area.height() - handle_h).max(1.0) * max_scroll;
        app.arr_scroll_y = (app.arr_scroll_y + delta).clamp(0.0, max_scroll);
    }
    let _ = timeline_w;
}

fn draw_returns(
    ui: &mut Ui,
    app: &mut App,
    project: &ProjectView,
    rect: Rect,
    timeline_w: f32,
    tracks_bottom: f32,
    returns_bottom: f32,
) {
    let painter = ui.painter().clone();
    let base_y = tracks_bottom + 3.0;
    // Return lanes.
    let lane = Rect::from_min_max(
        Pos2::new(rect.left(), base_y),
        Pos2::new(rect.left() + timeline_w, returns_bottom),
    );
    painter.rect_filled(lane, egui::CornerRadius::ZERO, theme::SURFACE_DARK);
    for row in 0..3 {
        let y = base_y + row as f32 * RETURN_H;
        let r = Rect::from_min_size(
            Pos2::new(rect.left(), y),
            Vec2::new(timeline_w, RETURN_H - 1.0),
        );
        painter.rect_filled(
            r,
            egui::CornerRadius::ZERO,
            if row % 2 == 0 {
                Color32::from_rgb(0x9B, 0x9B, 0x9B)
            } else {
                Color32::from_rgb(0xA5, 0xA5, 0xA5)
            },
        );
        painter.hline(
            r.x_range(),
            r.bottom() - 0.5,
            Stroke::new(1.0, theme::BORDER_SOFT),
        );
    }

    let header_left = rect.left() + timeline_w;
    let header = Rect::from_min_max(
        Pos2::new(header_left, base_y),
        Pos2::new(rect.right(), returns_bottom),
    );
    painter.rect_filled(header, egui::CornerRadius::ZERO, theme::SURFACE);
    for row in 0..3 {
        let Some(ret) = project.returns.get(row) else {
            continue;
        };
        let y = base_y + row as f32 * RETURN_H;
        let r = Rect::from_min_size(
            Pos2::new(header_left, y),
            Vec2::new(header.width(), RETURN_H - 1.0),
        );
        painter.rect_filled(r, egui::CornerRadius::ZERO, theme::SURFACE);
        let label = Rect::from_min_size(Pos2::new(header_left, y), Vec2::new(94.0, RETURN_H - 1.0));
        painter.rect_filled(label, egui::CornerRadius::ZERO, ret.color);
        painter.text(
            Pos2::new(label.left() + 8.0, label.center().y),
            Align2::LEFT_CENTER,
            &ret.name,
            FontId::proportional(9.0),
            theme::TEXT,
        );
        let letter = Rect::from_min_size(
            Pos2::new(label.right() + 6.0, y + 1.0),
            Vec2::new(if row == 2 { 38.0 } else { 22.0 }, 15.0),
        );
        theme::value_field(
            ui,
            letter,
            &ret.letter,
            Align2::CENTER_CENTER,
            8.8,
            if row == 2 {
                Color32::from_rgb(0x9B, 0xC9, 0xE8)
            } else {
                theme::ACCENT_FIELD
            },
        );
        let s_rect = Rect::from_min_size(
            Pos2::new(letter.right() + 4.0, y + 1.0),
            Vec2::new(17.0, 15.0),
        );
        theme::bevel_button(ui, s_rect, "S", None, false, true);
        let post_rect = Rect::from_min_size(
            Pos2::new(s_rect.right() + 4.0, y + 1.0),
            Vec2::new(36.0, 15.0),
        );
        theme::bevel_button(ui, post_rect, "Post", None, false, true);
        let id = ui.make_persistent_id(("return_header", row));
        let response = ui.interact(r, id, Sense::click());
        if response.clicked() {
            app.selected_track = Some(project.tracks.len() + row);
        }
    }
}

fn draw_time_ruler(
    ui: &mut Ui,
    project: &ProjectView,
    app: &App,
    rect: Rect,
    timeline_w: f32,
    top: f32,
) {
    let painter = ui.painter().clone();
    let area = Rect::from_min_size(
        Pos2::new(rect.left(), top),
        Vec2::new(rect.width(), TIME_RULER_H),
    );
    painter.rect_filled(area, egui::CornerRadius::ZERO, theme::SURFACE);
    painter.hline(
        area.x_range(),
        area.top() + 0.5,
        Stroke::new(1.0, theme::BORDER_SOFT),
    );
    painter.hline(
        area.x_range(),
        area.bottom() - 0.5,
        Stroke::new(1.0, theme::BORDER_SOFT),
    );
    let timeline = Rect::from_min_size(
        Pos2::new(rect.left(), top),
        Vec2::new(timeline_w, TIME_RULER_H),
    );
    let p = painter.with_clip_rect(timeline);
    let bpm = app.effective_bpm().max(1.0);
    let px_per_second = BAR_W as f64 / 4.0 * bpm / 60.0;
    let mut interval = 10.0;
    while interval * px_per_second < 44.0 {
        interval += 10.0;
    }
    let total_seconds = (timeline_w as f64 / px_per_second) + interval;
    let mut seconds = 0.0;
    while seconds <= total_seconds {
        let x = timeline.left() + (seconds * px_per_second) as f32 + 18.0;
        if x <= timeline.right() {
            p.vline(
                x.round(),
                (top + 4.0)..=(top + TIME_RULER_H - 1.0),
                Stroke::new(1.0, Color32::from_rgb(0x72, 0x72, 0x72)),
            );
            let minutes = (seconds / 60.0).floor() as i64;
            let secs = (seconds % 60.0) as i64;
            p.text(
                Pos2::new(x + 2.0, top + 7.0),
                Align2::LEFT_CENTER,
                format!("{minutes}:{secs:02}"),
                FontId::proportional(8.5),
                Color32::from_rgb(0x3D, 0x3D, 0x3D),
            );
        }
        seconds += interval;
    }
    let header_left = rect.left() + timeline_w;
    let mut x = header_left + 7.0;
    let one_bar = Rect::from_min_size(Pos2::new(x, top + 2.0), Vec2::new(32.0, TIME_RULER_H - 4.0));
    theme::value_field(ui, one_bar, "1/1", Align2::CENTER_CENTER, 8.5, theme::FIELD);
    x += 35.0;
    let speed = Rect::from_min_size(Pos2::new(x, top + 2.0), Vec2::new(42.0, TIME_RULER_H - 4.0));
    theme::value_field(ui, speed, "1.00x", Align2::CENTER_CENTER, 8.5, theme::FIELD);
    x += 45.0;
    let h = Rect::from_min_size(Pos2::new(x, top + 2.0), Vec2::new(17.0, TIME_RULER_H - 4.0));
    theme::bevel_button(ui, h, "H", None, false, true);
    x += 20.0;
    let w = Rect::from_min_size(Pos2::new(x, top + 2.0), Vec2::new(17.0, TIME_RULER_H - 4.0));
    theme::bevel_button(ui, w, "W", None, false, true);
    let _ = project;
}

fn draw_scrollbar(
    ui: &mut Ui,
    app: &mut App,
    project: &ProjectView,
    rect: Rect,
    timeline_w: f32,
    _tracks_bottom: f32,
) {
    let bar = Rect::from_min_max(
        Pos2::new(rect.left(), rect.bottom() - SCROLL_H),
        Pos2::new(rect.right(), rect.bottom()),
    );
    let painter = ui.painter().clone();
    painter.rect_filled(bar, egui::CornerRadius::ZERO, theme::SURFACE);
    painter.hline(
        bar.x_range(),
        bar.top() + 0.5,
        Stroke::new(1.0, theme::BORDER_SOFT),
    );
    painter.hline(
        bar.x_range(),
        bar.top() + 1.5,
        Stroke::new(1.0, theme::PANEL_HI),
    );
    let track = Rect::from_min_size(
        Pos2::new(rect.left() + 18.0, bar.top() + 4.0),
        Vec2::new(timeline_w - 36.0, 6.0),
    );
    painter.rect_filled(
        track,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0x8B, 0x8B, 0x8B),
    );
    let total_bars = ((project.duration_beats / 4.0) + 64.0).max(64.0);
    let visible_bars = timeline_w as f64 / BAR_W as f64;
    let handle_w = (visible_bars / total_bars).clamp(0.08, 1.0) as f32 * track.width();
    let max_offset = (total_bars - visible_bars).max(0.0);
    let frac = if max_offset > 0.0 {
        (app.h_offset_bars / max_offset).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };
    let handle = Rect::from_min_size(
        Pos2::new(
            track.left() + frac * (track.width() - handle_w),
            track.top(),
        ),
        Vec2::new(handle_w, track.height()),
    );
    painter.rect_filled(
        handle,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0x6D, 0x6D, 0x6D),
    );
    theme::frame_rect(&painter, handle, theme::BORDER_SOFT);
    let response = ui.interact(
        handle,
        ui.make_persistent_id("h_scroll_handle"),
        Sense::drag(),
    );
    if response.dragged() {
        let delta = response.drag_delta().x / track.width().max(1.0) * total_bars as f32;
        app.h_offset_bars = (app.h_offset_bars + delta as f64).clamp(0.0, max_offset);
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    if timeline_w + 340.0 < rect.width() {
        let grid = Rect::from_min_size(
            Pos2::new(rect.left() + timeline_w + 7.0, bar.top() + 2.0),
            Vec2::new(28.0, bar.height() - 4.0),
        );
        theme::value_field(ui, grid, "1/2", Align2::CENTER_CENTER, 8.5, theme::FIELD);
        let minus = Rect::from_min_size(
            Pos2::new(grid.right() + 3.0, bar.top() + 3.0),
            Vec2::new(14.0, bar.height() - 6.0),
        );
        theme::icon_button(ui, minus, icons::REMOVE, false, true);
        let plus = Rect::from_min_size(
            Pos2::new(minus.right() + 2.0, bar.top() + 3.0),
            Vec2::new(14.0, bar.height() - 6.0),
        );
        theme::icon_button(ui, plus, icons::ADD, false, true);
    }
}

fn draw_playhead(ui: &egui::Ui, app: &App, _project: &ProjectView, timeline_w: f32, _top: f32) {
    let rect = ui.max_rect();
    let x = rect.left() + 18.0 + (app.playhead_beats / 4.0) as f32 * BAR_W;
    if x < rect.left() || x > rect.left() + timeline_w {
        return;
    }
    let painter = ui.painter();
    painter.vline(
        x,
        (rect.top() + HEADER_H)..=(rect.bottom() - SCROLL_H),
        Stroke::new(1.0, Color32::from_rgb(0x1B, 0x1B, 0x1B)),
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(x - 4.0, rect.top() + HEADER_H - 10.0),
            Pos2::new(x + 4.0, rect.top() + HEADER_H - 10.0),
            Pos2::new(x, rect.top() + HEADER_H - 2.0),
        ],
        theme::TEXT,
        Stroke::NONE,
    ));
}
