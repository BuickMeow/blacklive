use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2,
};

use super::App;
use super::actions::UiActions;
use super::icons;
use super::project::{ProjectView, TrackKind};
use super::theme;

const HEADER_H: f32 = 26.0;
const SLOT_H: f32 = 24.0;
const SLOTS: usize = 7;
const STOP_H: f32 = 20.0;
const RET_W: f32 = 100.0;

pub(crate) fn show(ui: &mut Ui, app: &mut App, project: &ProjectView, _actions: &mut UiActions) {
    let rect = ui.max_rect();
    let painter = ui.painter().clone();
    painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE_DARK);

    let tracks_area_w = (rect.width() - RET_W * 3.0).max(120.0);
    let count = project.tracks.len().max(1);
    let col_w = (tracks_area_w / count as f32).clamp(108.0, 200.0);
    let total_w = col_w * count as f32;
    let max_scroll = (total_w - tracks_area_w).max(0.0);

    if let Some(pos) = ui.input(|input| input.pointer.hover_pos())
        && rect.contains(pos)
    {
        let (scroll, modifiers) = ui.input(|input| (input.smooth_scroll_delta, input.modifiers));
        if modifiers.shift || scroll.x.abs() > scroll.y.abs() {
            app.session_scroll_x = (app.session_scroll_x - scroll.x).clamp(0.0, max_scroll);
        }
    }
    app.session_scroll_x = app.session_scroll_x.clamp(0.0, max_scroll);

    let clip_rect = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top()),
        Pos2::new(rect.left() + tracks_area_w, rect.bottom()),
    );
    let p = painter.with_clip_rect(clip_rect);
    for (index, track) in project.tracks.iter().enumerate() {
        let x = rect.left() + index as f32 * col_w - app.session_scroll_x;
        if x + col_w < rect.left() || x > rect.left() + tracks_area_w {
            continue;
        }
        draw_track_column(ui, app, project, track, index, x, col_w, rect, &p);
    }

    // Returns and master.
    let mut x = rect.left() + tracks_area_w;
    for (index, ret) in project.returns.iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(RET_W, rect.height()));
        draw_return_column(ui, app, project, ret, index, r);
        x += RET_W;
    }

    // Horizontal scrollbar for many tracks.
    if max_scroll > 0.0 && rect.height() > 16.0 {
        let bar = Rect::from_min_max(
            Pos2::new(rect.left(), rect.bottom() - 10.0),
            Pos2::new(rect.left() + tracks_area_w, rect.bottom()),
        );
        painter.rect_filled(bar, egui::CornerRadius::ZERO, theme::SURFACE);
        let frac = (app.session_scroll_x / max_scroll).clamp(0.0, 1.0);
        let handle_w = (tracks_area_w / total_w * tracks_area_w).max(40.0);
        let handle = Rect::from_min_size(
            Pos2::new(
                bar.left() + frac * (bar.width() - handle_w),
                bar.top() + 2.0,
            ),
            Vec2::new(handle_w, 6.0),
        );
        painter.rect_filled(
            handle,
            egui::CornerRadius::ZERO,
            Color32::from_rgb(0x6D, 0x6D, 0x6D),
        );
        theme::frame_rect(&painter, handle, theme::BORDER_SOFT);
        let response = ui.interact(
            handle,
            ui.make_persistent_id("session_h_scroll"),
            Sense::drag(),
        );
        if response.dragged() {
            app.session_scroll_x =
                (app.session_scroll_x + response.drag_delta().x).clamp(0.0, max_scroll);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_track_column(
    ui: &mut Ui,
    app: &mut App,
    project: &ProjectView,
    track: &super::project::TrackView,
    index: usize,
    x: f32,
    w: f32,
    rect: Rect,
    p: &egui::Painter,
) {
    let col = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, rect.height()));
    let selected = app.selected_track == Some(index);
    p.rect_filled(
        col,
        egui::CornerRadius::ZERO,
        if selected {
            Color32::from_rgb(0xA9, 0xA9, 0xA9)
        } else {
            theme::SURFACE
        },
    );
    p.vline(
        col.left() + 0.5,
        col.y_range(),
        Stroke::new(1.0, theme::BORDER_SOFT),
    );

    let header = Rect::from_min_size(col.min, Vec2::new(w, HEADER_H));
    p.rect_filled(header, egui::CornerRadius::ZERO, track.color);
    p.hline(
        header.x_range(),
        header.bottom() - 0.5,
        Stroke::new(1.0, theme::mix(track.color, Color32::BLACK, 0.35)),
    );
    p.circle_stroke(
        Pos2::new(header.left() + 12.0, header.center().y),
        6.5,
        Stroke::new(1.0, theme::TEXT),
    );
    p.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(header.left() + 9.0, header.center().y - 2.0),
            Pos2::new(header.left() + 15.0, header.center().y - 2.0),
            Pos2::new(header.left() + 12.0, header.center().y + 2.5),
        ],
        theme::TEXT,
        Stroke::NONE,
    ));
    theme::label_at(
        &p.with_clip_rect(header.shrink2(Vec2::new(20.0, 0.0))),
        Rect::from_min_max(
            Pos2::new(header.left() + 20.0, header.top()),
            Pos2::new(header.right() - 24.0, header.bottom()),
        ),
        &track.name,
        10.5,
        theme::TEXT,
        Align2::LEFT_CENTER,
    );
    p.circle_filled(
        Pos2::new(header.right() - 12.0, header.center().y),
        4.0,
        if track.armed {
            Color32::from_rgb(0xC4, 0x28, 0x28)
        } else {
            Color32::from_rgb(0x51, 0x51, 0x51)
        },
    );

    let header_response = ui.interact(
        header,
        ui.make_persistent_id(("session_header", index)),
        Sense::click(),
    );
    if header_response.clicked() {
        app.selected_track = Some(index);
    }
    if header_response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    let slots_top = rect.top() + HEADER_H;
    for slot_index in 0..SLOTS {
        let slot = Rect::from_min_size(
            Pos2::new(col.left() + 2.0, slots_top + slot_index as f32 * SLOT_H),
            Vec2::new(w - 4.0, SLOT_H - 2.0),
        );
        if slot_index == 0
            && let Some(clip) = track.clips.first()
        {
            draw_clip_slot(p, slot, clip, &track.name);
        } else {
            p.rect_filled(
                slot,
                egui::CornerRadius::ZERO,
                Color32::from_rgb(0xA0, 0xA0, 0xA0),
            );
            p.hline(
                slot.x_range(),
                slot.bottom() - 0.5,
                Stroke::new(1.0, Color32::from_rgb(0x86, 0x86, 0x86)),
            );
            p.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(slot.left() + 5.0, slot.center().y - 3.0),
                    Pos2::new(slot.left() + 11.0, slot.center().y),
                    Pos2::new(slot.left() + 5.0, slot.center().y + 3.0),
                ],
                Color32::from_rgb(0x6A, 0x6A, 0x6A),
                Stroke::NONE,
            ));
        }
        let response = ui.interact(
            slot,
            ui.make_persistent_id(("session_slot", index, slot_index)),
            Sense::click(),
        );
        if response.clicked() {
            app.selected_track = Some(index);
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
    }

    let stop_y = slots_top + SLOTS as f32 * SLOT_H;
    let stop = Rect::from_min_size(
        Pos2::new(col.left() + 4.0, stop_y),
        Vec2::new(20.0, STOP_H - 3.0),
    );
    theme::icon_button(ui, stop, icons::STOP, false, true);

    let mixer_top = stop_y + STOP_H;
    draw_mixer(ui, app, track, index, col, mixer_top);
    let _ = project;
}

fn draw_clip_slot(p: &egui::Painter, slot: Rect, clip: &super::project::ClipView, name: &str) {
    p.rect_filled(slot, egui::CornerRadius::ZERO, clip.color);
    p.hline(
        slot.x_range(),
        slot.bottom() - 0.5,
        Stroke::new(1.0, theme::mix(clip.color, Color32::BLACK, 0.4)),
    );
    p.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(slot.left() + 5.0, slot.center().y - 3.0),
            Pos2::new(slot.left() + 11.0, slot.center().y),
            Pos2::new(slot.left() + 5.0, slot.center().y + 3.0),
        ],
        Color32::from_rgb(0x22, 0x22, 0x22),
        Stroke::NONE,
    ));
    theme::label_at(
        p,
        Rect::from_min_max(
            Pos2::new(slot.left() + 15.0, slot.top()),
            Pos2::new(slot.right() - 3.0, slot.bottom()),
        ),
        if clip.name.is_empty() {
            name
        } else {
            &clip.name
        },
        9.5,
        Color32::from_rgb(0x18, 0x18, 0x18),
        Align2::LEFT_CENTER,
    );
}

fn draw_mixer(
    ui: &mut Ui,
    app: &mut App,
    track: &super::project::TrackView,
    index: usize,
    col: Rect,
    top: f32,
) {
    let p = ui.painter().with_clip_rect(col);
    let x = col.left() + 6.0;
    let w = col.width() - 12.0;
    let is_midi = track.kind == TrackKind::Midi;

    p.rect_filled(
        Rect::from_min_max(Pos2::new(col.left(), top), col.max),
        egui::CornerRadius::ZERO,
        theme::SURFACE,
    );
    p.hline(
        col.x_range(),
        top + 0.5,
        Stroke::new(1.0, theme::BORDER_SOFT),
    );

    let label = |y: f32, text: &str| {
        p.text(
            Pos2::new(x, y),
            Align2::LEFT_CENTER,
            text,
            FontId::proportional(8.5),
            theme::TEXT_DIM,
        );
    };
    let mut y = top + 8.0;
    label(y, if is_midi { "MIDI From" } else { "Audio From" });
    y += 8.0;
    let from = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, 15.0));
    theme::value_field(
        ui,
        from,
        "All Ins",
        Align2::LEFT_CENTER,
        8.5,
        Color32::from_rgb(0xE1, 0xE1, 0xE1),
    );
    y += 16.0;
    let channels = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, 15.0));
    theme::value_field(
        ui,
        channels,
        if is_midi { "All Channels" } else { "Ext. In" },
        Align2::LEFT_CENTER,
        8.5,
        Color32::from_rgb(0xE1, 0xE1, 0xE1),
    );
    y += 20.0;
    label(y, "Monitor");
    y += 9.0;
    draw_monitor(
        &p,
        Rect::from_min_size(Pos2::new(x, y), Vec2::new(w.min(64.0), 15.0)),
    );
    y += 20.0;
    label(y, if is_midi { "MIDI To" } else { "Audio To" });
    y += 8.0;
    let to = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, 15.0));
    theme::value_field(
        ui,
        to,
        if is_midi { "No Output" } else { "Main" },
        Align2::LEFT_CENTER,
        8.5,
        Color32::from_rgb(0xE1, 0xE1, 0xE1),
    );

    // Sends.
    let sends_top = top + 150.0;
    label(sends_top, "Sends");
    draw_knob(&p, Pos2::new(x + 14.0, sends_top + 22.0), 9.0, 0.35, "A");
    draw_knob(&p, Pos2::new(x + 46.0, sends_top + 22.0), 9.0, 0.18, "B");

    // Pan + fader.
    let fader_top = top + 200.0;
    let fader_bottom = col.bottom() - 38.0;
    label(fader_top - 10.0, "Volume");
    draw_pan(&p, Pos2::new(x + 12.0, fader_top + 10.0), 8.0);
    let fader_x = x + 46.0;
    draw_fader(&p, fader_x, fader_top + 4.0, fader_bottom, 0.72);
    draw_meter(&p, fader_x + 12.0, fader_top + 4.0, fader_bottom, 0.54);
    // dB scale hints.
    for (i, text) in ["0", "6", "12", "24", "48"].iter().enumerate() {
        p.text(
            Pos2::new(fader_x + 20.0, fader_top + 4.0 + i as f32 * 18.0),
            Align2::LEFT_CENTER,
            *text,
            FontId::proportional(7.0),
            theme::TEXT_DIM,
        );
    }

    // Solo / arm.
    let by = col.bottom() - 20.0;
    let s_rect = Rect::from_min_size(Pos2::new(x, by), Vec2::new(20.0, 15.0));
    if theme::bevel_button(
        ui,
        s_rect,
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
    let a_rect = Rect::from_min_size(Pos2::new(x + 24.0, by), Vec2::new(20.0, 15.0));
    if theme::icon_button(
        ui,
        a_rect,
        icons::FIBER_MANUAL_RECORD,
        app.armed_tracks.contains(&index),
        true,
    )
    .clicked()
        && !app.armed_tracks.insert(index)
    {
        app.armed_tracks.remove(&index);
    }
}

fn draw_monitor(p: &egui::Painter, rect: Rect) {
    let widths = [18.0, 26.0, 20.0];
    let labels = ["In", "Auto", "Off"];
    let mut x = rect.left();
    for (label, w) in labels.iter().zip(widths) {
        let r = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, rect.height()));
        p.rect_filled(
            r,
            egui::CornerRadius::ZERO,
            if *label == "Auto" {
                theme::ACCENT_FIELD
            } else {
                Color32::from_rgb(0xD8, 0xD8, 0xD8)
            },
        );
        theme::frame_rect(p, r, theme::BORDER_SOFT);
        p.text(
            r.center(),
            Align2::CENTER_CENTER,
            *label,
            FontId::proportional(8.0),
            theme::TEXT,
        );
        x += w;
    }
}

fn draw_knob(p: &egui::Painter, center: Pos2, radius: f32, value: f32, label: &str) {
    p.circle_filled(center, radius, Color32::from_rgb(0xB0, 0xB0, 0xB0));
    p.circle_stroke(center, radius, Stroke::new(1.0, theme::BORDER_SOFT));
    let angle = -std::f32::consts::FRAC_PI_2 + value * std::f32::consts::TAU;
    p.line_segment(
        [
            center,
            Pos2::new(
                center.x + angle.cos() * (radius - 2.0),
                center.y + angle.sin() * (radius - 2.0),
            ),
        ],
        Stroke::new(1.5, theme::TEXT),
    );
    p.text(
        Pos2::new(center.x, center.y + radius + 7.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(8.0),
        theme::TEXT_DIM,
    );
}

fn draw_pan(p: &egui::Painter, center: Pos2, radius: f32) {
    p.circle_filled(center, radius, Color32::from_rgb(0xB0, 0xB0, 0xB0));
    p.circle_stroke(center, radius, Stroke::new(1.0, theme::BORDER_SOFT));
    p.line_segment(
        [center, Pos2::new(center.x, center.y - radius + 2.0)],
        Stroke::new(1.5, theme::TEXT),
    );
}

fn draw_fader(p: &egui::Painter, x: f32, top: f32, bottom: f32, value: f32) {
    if bottom <= top + 10.0 {
        return;
    }
    p.rect_filled(
        Rect::from_min_size(Pos2::new(x - 2.0, top), Vec2::new(4.0, bottom - top)),
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0x4F, 0x4F, 0x4F),
    );
    let y = bottom - (bottom - top) * value.clamp(0.0, 1.0);
    let handle = Rect::from_center_size(Pos2::new(x, y), Vec2::new(16.0, 8.0));
    p.rect_filled(
        handle,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0xD6, 0xD6, 0xD6),
    );
    theme::frame_rect(p, handle, theme::BORDER);
    p.hline(
        handle.x_range(),
        handle.center().y,
        Stroke::new(1.0, theme::BORDER_SOFT),
    );
}

fn draw_meter(p: &egui::Painter, x: f32, top: f32, bottom: f32, value: f32) {
    if bottom <= top + 10.0 {
        return;
    }
    let rect = Rect::from_min_size(Pos2::new(x, top), Vec2::new(3.0, bottom - top));
    p.rect_filled(rect, egui::CornerRadius::ZERO, theme::METER_BG);
    let h = rect.height() * value.clamp(0.0, 1.0);
    p.rect_filled(
        Rect::from_min_size(
            Pos2::new(rect.left(), rect.bottom() - h),
            Vec2::new(rect.width(), h),
        ),
        egui::CornerRadius::ZERO,
        theme::METER_GREEN,
    );
}

fn draw_return_column(
    ui: &mut Ui,
    app: &mut App,
    project: &ProjectView,
    ret: &super::project::ReturnView,
    index: usize,
    rect: Rect,
) {
    let p = ui.painter().with_clip_rect(rect);
    p.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE);
    p.vline(
        rect.left() + 0.5,
        rect.y_range(),
        Stroke::new(1.0, theme::BORDER_SOFT),
    );
    let header = Rect::from_min_size(rect.min, Vec2::new(rect.width(), HEADER_H));
    p.rect_filled(header, egui::CornerRadius::ZERO, ret.color);
    p.hline(
        header.x_range(),
        header.bottom() - 0.5,
        Stroke::new(1.0, theme::mix(ret.color, Color32::BLACK, 0.35)),
    );
    theme::label_at(
        &p,
        Rect::from_min_max(
            Pos2::new(header.left() + 8.0, header.top()),
            Pos2::new(header.right() - 4.0, header.bottom()),
        ),
        &ret.name,
        9.5,
        theme::TEXT,
        Align2::LEFT_CENTER,
    );

    let slots_top = rect.top() + HEADER_H;
    for slot_index in 0..SLOTS {
        let slot = Rect::from_min_size(
            Pos2::new(rect.left() + 3.0, slots_top + slot_index as f32 * SLOT_H),
            Vec2::new(rect.width() - 6.0, SLOT_H - 2.0),
        );
        p.rect_filled(
            slot,
            egui::CornerRadius::ZERO,
            Color32::from_rgb(0xA0, 0xA0, 0xA0),
        );
        p.hline(
            slot.x_range(),
            slot.bottom() - 0.5,
            Stroke::new(1.0, Color32::from_rgb(0x86, 0x86, 0x86)),
        );
    }
    let stop_y = slots_top + SLOTS as f32 * SLOT_H;
    let stop = Rect::from_min_size(
        Pos2::new(rect.left() + 5.0, stop_y),
        Vec2::new(20.0, STOP_H - 3.0),
    );
    theme::icon_button(ui, stop, icons::STOP, false, true);

    let top = stop_y + STOP_H;
    let fader_top = top + 180.0;
    let fader_bottom = rect.bottom() - 34.0;
    let letter = Rect::from_min_size(
        Pos2::new(rect.left() + 8.0, top + 14.0),
        Vec2::new(rect.width() - 16.0, 18.0),
    );
    theme::value_field(
        ui,
        letter,
        &ret.letter,
        Align2::CENTER_CENTER,
        9.5,
        if index == 2 {
            Color32::from_rgb(0x9B, 0xC9, 0xE8)
        } else {
            theme::ACCENT_FIELD
        },
    );
    draw_fader(&p, rect.center().x, fader_top, fader_bottom, 0.72);
    draw_meter(&p, rect.center().x + 12.0, fader_top, fader_bottom, 0.58);
    for (i, text) in ["0", "6", "12", "24", "48"].iter().enumerate() {
        p.text(
            Pos2::new(rect.center().x + 20.0, fader_top + i as f32 * 18.0),
            Align2::LEFT_CENTER,
            *text,
            FontId::proportional(7.0),
            theme::TEXT_DIM,
        );
    }
    let s_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 8.0, rect.bottom() - 24.0),
        Vec2::new(18.0, 15.0),
    );
    theme::bevel_button(ui, s_rect, "S", None, false, true);
    let post = Rect::from_min_size(
        Pos2::new(s_rect.right() + 4.0, s_rect.top()),
        Vec2::new(38.0, 15.0),
    );
    theme::bevel_button(ui, post, "Post", None, false, true);

    let response = ui.interact(
        rect,
        ui.make_persistent_id(("session_return", index)),
        Sense::click(),
    );
    if response.clicked() {
        app.selected_track = Some(project.tracks.len() + index);
    }
}
