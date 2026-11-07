use std::sync::Arc;

use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontData, FontDefinitions, FontFamily, FontId,
    Margin, Painter, Pos2, Rect, Response, RichText, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2,
    epaint::text::LayoutJob, text::TextWrapping,
};

use super::icons;

pub(crate) const TITLE_BG: Color32 = Color32::from_rgb(0x81, 0x81, 0x81);
pub(crate) const SURFACE: Color32 = Color32::from_rgb(0xA5, 0xA5, 0xA5);
pub(crate) const SURFACE_DARK: Color32 = Color32::from_rgb(0x9B, 0x9B, 0x9B);
pub(crate) const SURFACE_LIGHT: Color32 = Color32::from_rgb(0xB0, 0xB0, 0xB0);
pub(crate) const PANEL_HI: Color32 = Color32::from_rgb(0xBD, 0xBD, 0xBD);
pub(crate) const BORDER: Color32 = Color32::from_rgb(0x4F, 0x4F, 0x4F);
pub(crate) const BORDER_SOFT: Color32 = Color32::from_rgb(0x6E, 0x6E, 0x6E);
pub(crate) const FIELD: Color32 = Color32::from_rgb(0xDF, 0xDF, 0xDF);
pub(crate) const TEXT: Color32 = Color32::from_rgb(0x16, 0x16, 0x16);
pub(crate) const TEXT_DIM: Color32 = Color32::from_rgb(0x5B, 0x5B, 0x5B);
pub(crate) const TEXT_DISABLED: Color32 = Color32::from_rgb(0x7A, 0x7A, 0x7A);
pub(crate) const ACCENT_DARK: Color32 = Color32::from_rgb(0xD8, 0x93, 0x00);
pub(crate) const ACCENT_FIELD: Color32 = Color32::from_rgb(0xF4, 0xBC, 0x41);
pub(crate) const SELECTION: Color32 = Color32::from_rgb(0xCD, 0xF8, 0xFF);
pub(crate) const METER_BG: Color32 = Color32::from_rgb(0x5B, 0x5B, 0x5B);
pub(crate) const METER_GREEN: Color32 = Color32::from_rgb(0x76, 0xB8, 0x50);

pub(crate) const CONTROL_H: f32 = 18.0;
pub(crate) const TRACK_H: f32 = 88.0;
pub(crate) const HEADER_W: f32 = 291.0;
pub(crate) const BROWSER_W: f32 = 400.0;

pub(crate) const TRACK_COLORS: [Color32; 16] = [
    Color32::from_rgb(0xD4, 0x5D, 0x9E),
    Color32::from_rgb(0xF0, 0x99, 0xA7),
    Color32::from_rgb(0xEB, 0x4C, 0xCE),
    Color32::from_rgb(0xC4, 0x9B, 0x40),
    Color32::from_rgb(0x7D, 0xA7, 0xD9),
    Color32::from_rgb(0x8E, 0xCF, 0x83),
    Color32::from_rgb(0xE1, 0x8B, 0x5A),
    Color32::from_rgb(0xB5, 0x8B, 0xD9),
    Color32::from_rgb(0x6E, 0xC6, 0xC4),
    Color32::from_rgb(0xD9, 0xC4, 0x6A),
    Color32::from_rgb(0xE0, 0x70, 0x70),
    Color32::from_rgb(0x9F, 0xA8, 0xDA),
    Color32::from_rgb(0xA1, 0xC6, 0xA1),
    Color32::from_rgb(0xE3, 0xA6, 0xD0),
    Color32::from_rgb(0xA9, 0xA9, 0xA9),
    Color32::from_rgb(0xC7, 0xB3, 0x8C),
];

pub(crate) const RETURN_A: Color32 = Color32::from_rgb(0xAD, 0x7A, 0xC1);
pub(crate) const RETURN_B: Color32 = Color32::from_rgb(0x36, 0x7B, 0xBB);
pub(crate) const MAIN: Color32 = Color32::from_rgb(0xCA, 0x72, 0xDE);

pub(crate) fn apply(ctx: &egui::Context) {
    install_fonts(ctx);

    ctx.set_theme(egui::ThemePreference::Light);
    let mut style = (*ctx.style_of(egui::Theme::Light)).clone();
    style.visuals = egui::Visuals::light();
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.panel_fill = SURFACE;
    style.visuals.window_fill = SURFACE;
    style.visuals.extreme_bg_color = FIELD;
    style.visuals.faint_bg_color = SURFACE_LIGHT;
    style.visuals.selection.bg_fill = SELECTION;
    style.visuals.selection.stroke = Stroke::new(1.0, TEXT);
    style.visuals.hyperlink_color = Color32::from_rgb(0x1A, 0x5C, 0xA8);
    style.visuals.window_corner_radius = egui::CornerRadius::ZERO;
    style.visuals.menu_corner_radius = egui::CornerRadius::ZERO;
    style.visuals.window_stroke = Stroke::new(1.0, BORDER);
    style.visuals.widgets.noninteractive.bg_fill = SURFACE;
    style.visuals.widgets.noninteractive.weak_bg_fill = SURFACE;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;

    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::ZERO;
        widget.expansion = 0.0;
        widget.fg_stroke = Stroke::new(1.0, TEXT);
    }
    style.visuals.widgets.inactive.bg_fill = SURFACE;
    style.visuals.widgets.inactive.weak_bg_fill = SURFACE;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_SOFT);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(0xB8, 0xB8, 0xB8);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xB8, 0xB8, 0xB8);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(0x91, 0x91, 0x91);
    style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(0x91, 0x91, 0x91);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, BORDER);
    style.visuals.widgets.open.bg_fill = Color32::from_rgb(0xBE, 0xBE, 0xBE);
    style.visuals.widgets.open.weak_bg_fill = Color32::from_rgb(0xBE, 0xBE, 0xBE);
    style.visuals.widgets.open.bg_stroke = Stroke::new(1.0, BORDER);

    style.spacing.item_spacing = Vec2::new(4.0, 3.0);
    style.spacing.button_padding = Vec2::new(5.0, 2.0);
    style.spacing.interact_size = Vec2::new(20.0, CONTROL_H);
    style.spacing.slider_width = 80.0;
    style.spacing.combo_width = 70.0;
    style.spacing.scroll.bar_width = 10.0;
    style.spacing.scroll.floating = false;
    style.spacing.menu_margin = Margin::same(2);
    style.spacing.tooltip_width = 360.0;

    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::new(12.5, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(11.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::new(10.5, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::new(9.5, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::new(10.0, FontFamily::Monospace),
    );

    ctx.set_style_of(egui::Theme::Light, style.clone());
    ctx.set_style_of(egui::Theme::Dark, style);
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Material Icons lives in its own family so icon glyphs never fight with CJK.
    fonts.font_data.insert(
        "material_icons".to_owned(),
        Arc::new(icons::material_font_data()),
    );
    fonts.families.insert(
        FontFamily::Name(icons::MATERIAL_FAMILY.into()),
        vec!["material_icons".to_owned()],
    );

    // Chinese UI text. Prefer fonts that exist on a normal Live installation,
    // then fall back to the system CJK face.
    for (name, path) in [
        (
            "cjk_live",
            "/Applications/Ableton Live 12 Suite.app/Contents/App-Resources/Fonts/NotoSansCJKsc-Regular.otf",
        ),
        ("cjk_hiragino", "/System/Library/Fonts/Hiragino Sans GB.ttc"),
        (
            "cjk_arial",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ),
    ] {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .push(name.to_owned());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .push(name.to_owned());
        break;
    }

    ctx.set_fonts(fonts);
}

pub(crate) fn mix(a: Color32, b: Color32, b_amount: f32) -> Color32 {
    let t = b_amount.clamp(0.0, 1.0);
    let a_w = 1.0 - t;
    Color32::from_rgb(
        (a.r() as f32 * a_w + b.r() as f32 * t).round() as u8,
        (a.g() as f32 * a_w + b.g() as f32 * t).round() as u8,
        (a.b() as f32 * a_w + b.b() as f32 * t).round() as u8,
    )
}

pub(crate) fn frame_rect(painter: &Painter, rect: Rect, color: Color32) {
    painter.rect_stroke(
        rect,
        egui::CornerRadius::ZERO,
        Stroke::new(1.0, color),
        StrokeKind::Inside,
    );
}

pub(crate) fn bevel_button(
    ui: &mut Ui,
    rect: Rect,
    label: &str,
    icon: Option<&str>,
    active: bool,
    enabled: bool,
) -> Response {
    let id = ui.next_auto_id();
    let response = ui.interact(rect, id, Sense::click());
    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();

    let (top, _bottom, text_color, border) = if !enabled {
        (
            Color32::from_rgb(0x8E, 0x8E, 0x8E),
            Color32::from_rgb(0x8E, 0x8E, 0x8E),
            TEXT_DISABLED,
            Color32::from_rgb(0x6E, 0x6E, 0x6E),
        )
    } else if active {
        (
            ACCENT_FIELD,
            ACCENT_FIELD,
            TEXT,
            Color32::from_rgb(0x7A, 0x59, 0x00),
        )
    } else if pressed {
        (
            Color32::from_rgb(0x8F, 0x8F, 0x8F),
            Color32::from_rgb(0x8F, 0x8F, 0x8F),
            TEXT,
            BORDER,
        )
    } else if hovered {
        (
            Color32::from_rgb(0xB8, 0xB8, 0xB8),
            Color32::from_rgb(0xB8, 0xB8, 0xB8),
            TEXT,
            BORDER,
        )
    } else {
        (
            Color32::from_rgb(0xA5, 0xA5, 0xA5),
            Color32::from_rgb(0xA5, 0xA5, 0xA5),
            TEXT,
            BORDER_SOFT,
        )
    };

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, egui::CornerRadius::ZERO, top);
        painter.hline(
            rect.x_range().shrink(1.0),
            rect.top() + 1.0,
            Stroke::new(1.0, if active { ACCENT_FIELD } else { PANEL_HI }),
        );
        painter.hline(
            rect.x_range().shrink(1.0),
            rect.bottom() - 1.0,
            Stroke::new(1.0, mix(top, Color32::BLACK, 0.18)),
        );
        frame_rect(painter, rect, border);
        draw_button_content(ui, rect, label, icon, text_color, active);
    }

    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

pub(crate) fn flat_button(
    ui: &mut Ui,
    rect: Rect,
    label: &str,
    icon: Option<&str>,
    active: bool,
    enabled: bool,
) -> Response {
    let id = ui.next_auto_id();
    let response = ui.interact(rect, id, Sense::click());
    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();
    let fill = if !enabled {
        Color32::from_rgb(0x8E, 0x8E, 0x8E)
    } else if active {
        ACCENT_FIELD
    } else if pressed {
        Color32::from_rgb(0x8F, 0x8F, 0x8F)
    } else if hovered {
        Color32::from_rgb(0xB8, 0xB8, 0xB8)
    } else {
        Color32::from_rgb(0xA5, 0xA5, 0xA5)
    };
    let text = if enabled { TEXT } else { TEXT_DISABLED };
    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::ZERO, fill);
        draw_button_content(ui, rect, label, icon, text, active);
    }
    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

pub(crate) fn flat_icon_button(
    ui: &mut Ui,
    rect: Rect,
    icon: &str,
    active: bool,
    enabled: bool,
) -> Response {
    flat_button(ui, rect, "", Some(icon), active, enabled)
}

pub(crate) fn blank_button(ui: &mut Ui, rect: Rect) -> Response {
    let response = ui.interact(rect, ui.next_auto_id(), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

pub(crate) fn flat_field(painter: &Painter, rect: Rect, text: &str, align: Align2, size: f32) {
    painter.rect_filled(
        rect,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0xA5, 0xA5, 0xA5),
    );
    let clipped = painter.with_clip_rect(rect.shrink2(Vec2::new(4.0, 0.0)));
    clipped.text(
        match align {
            Align2::LEFT_CENTER => Pos2::new(rect.left() + 6.0, rect.center().y),
            Align2::RIGHT_CENTER => Pos2::new(rect.right() - 6.0, rect.center().y),
            _ => rect.center(),
        },
        align,
        text,
        FontId::proportional(size),
        TEXT,
    );
}

pub(crate) fn flat_text_edit_at(
    ui: &mut Ui,
    rect: Rect,
    buffer: &mut String,
    hint: &str,
    size: f32,
    align: Align,
) -> Response {
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0xA5, 0xA5, 0xA5),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(5.0, 1.0)))
            .layout(egui::Layout::left_to_right(Align::Center)),
    );
    child.style_mut().visuals.extreme_bg_color = Color32::from_rgb(0xA5, 0xA5, 0xA5);
    child.style_mut().visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    child.style_mut().visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    child.style_mut().visuals.widgets.active.bg_stroke = Stroke::NONE;
    child.style_mut().visuals.override_text_color = Some(TEXT);
    child.style_mut().spacing.text_edit_width = rect.width();
    let response = child.add(
        TextEdit::singleline(buffer)
            .frame(egui::Frame::NONE)
            .margin(Margin::ZERO)
            .font(FontId::proportional(size))
            .text_color(TEXT)
            .align(Align2::new(align, Align::Center))
            .hint_text(RichText::new(hint).color(TEXT_DISABLED).size(size))
            .min_size(Vec2::new(rect.width() - 10.0, rect.height() - 2.0)),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Text);
    }
    response
}

pub(crate) fn icon_button(
    ui: &mut Ui,
    rect: Rect,
    icon: &str,
    active: bool,
    enabled: bool,
) -> Response {
    bevel_button(ui, rect, "", Some(icon), active, enabled)
}

fn draw_button_content(
    ui: &Ui,
    rect: Rect,
    label: &str,
    icon: Option<&str>,
    color: Color32,
    _active: bool,
) {
    let painter = ui.painter().with_clip_rect(rect.shrink(1.0));
    let font = FontId::proportional(10.5);
    let icon_font = icons::font(13.0);
    let icon_w = icon
        .map(|i| {
            painter
                .layout_no_wrap(i.to_owned(), icon_font.clone(), color)
                .size()
                .x
        })
        .unwrap_or(0.0);
    let text_w = if label.is_empty() {
        0.0
    } else {
        painter
            .layout_no_wrap(label.to_owned(), font.clone(), color)
            .size()
            .x
    };
    let gap = if icon.is_some() && !label.is_empty() {
        4.0
    } else {
        0.0
    };
    let total = icon_w + gap + text_w;
    let mut x = rect.center().x - total * 0.5;
    if let Some(icon) = icon {
        let icon_rect = Rect::from_center_size(
            Pos2::new(x + icon_w * 0.5, rect.center().y),
            Vec2::new(icon_w + 2.0, rect.height()),
        );
        painter.text(
            icon_rect.center(),
            Align2::CENTER_CENTER,
            icon,
            icon_font,
            color,
        );
        x += icon_w + gap;
    }
    if !label.is_empty() {
        let text_rect = Rect::from_min_size(
            Pos2::new(x, rect.top()),
            Vec2::new(text_w + 2.0, rect.height()),
        );
        painter.text(
            Pos2::new(text_rect.left(), rect.center().y),
            Align2::LEFT_CENTER,
            label,
            font,
            color,
        );
    }
}

pub(crate) fn inset_field(painter: &Painter, rect: Rect, fill: Color32) {
    painter.rect_filled(rect, egui::CornerRadius::ZERO, fill);
    painter.hline(
        rect.x_range().shrink(1.0),
        rect.top() + 0.5,
        Stroke::new(1.0, Color32::from_rgb(0x75, 0x75, 0x75)),
    );
    painter.vline(
        rect.left() + 0.5,
        rect.y_range().shrink(1.0),
        Stroke::new(1.0, Color32::from_rgb(0x75, 0x75, 0x75)),
    );
    painter.hline(
        rect.x_range().shrink(1.0),
        rect.bottom() - 0.5,
        Stroke::new(1.0, Color32::from_rgb(0xF2, 0xF2, 0xF2)),
    );
    painter.vline(
        rect.right() - 0.5,
        rect.y_range().shrink(1.0),
        Stroke::new(1.0, Color32::from_rgb(0xF2, 0xF2, 0xF2)),
    );
    frame_rect(painter, rect, BORDER);
}

pub(crate) fn value_field(
    ui: &mut Ui,
    rect: Rect,
    text: &str,
    align: Align2,
    size: f32,
    fill: Color32,
) {
    let painter = ui.painter();
    inset_field(painter, rect, fill);
    let clipped = painter.with_clip_rect(rect.shrink(2.0));
    clipped.text(
        match align {
            Align2::LEFT_CENTER => Pos2::new(rect.left() + 5.0, rect.center().y),
            Align2::RIGHT_CENTER => Pos2::new(rect.right() - 5.0, rect.center().y),
            _ => rect.center(),
        },
        align,
        text,
        FontId::proportional(size),
        TEXT,
    );
}

pub(crate) fn text_edit_at(
    ui: &mut Ui,
    rect: Rect,
    buffer: &mut String,
    hint: &str,
    size: f32,
    align: Align,
) -> Response {
    inset_field(ui.painter(), rect, FIELD);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(5.0, 1.0)))
            .layout(egui::Layout::left_to_right(Align::Center)),
    );
    child.style_mut().visuals.extreme_bg_color = FIELD;
    child.style_mut().visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    child.style_mut().visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    child.style_mut().visuals.widgets.active.bg_stroke = Stroke::NONE;
    child.style_mut().visuals.override_text_color = Some(TEXT);
    child.style_mut().spacing.text_edit_width = rect.width();
    let response = child.add(
        TextEdit::singleline(buffer)
            .frame(egui::Frame::NONE)
            .margin(Margin::ZERO)
            .font(FontId::proportional(size))
            .text_color(TEXT)
            .align(Align2::new(align, Align::Center))
            .hint_text(RichText::new(hint).color(TEXT_DISABLED).size(size))
            .min_size(Vec2::new(rect.width() - 10.0, rect.height() - 2.0)),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Text);
    }
    response
}

pub(crate) fn drag_value_at(
    ui: &mut Ui,
    rect: Rect,
    value: &mut f64,
    suffix: &str,
    speed: f64,
    range: std::ops::RangeInclusive<f64>,
    decimals: usize,
) -> Response {
    inset_field(ui.painter(), rect, FIELD);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(4.0, 1.0)))
            .layout(egui::Layout::left_to_right(Align::Center)),
    );
    child.style_mut().visuals.extreme_bg_color = FIELD;
    child.style_mut().visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    child.style_mut().visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    child.style_mut().visuals.widgets.active.bg_stroke = Stroke::NONE;
    child
        .style_mut()
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(10.5));
    child.add(
        egui::DragValue::new(value)
            .speed(speed)
            .range(range)
            .suffix(suffix)
            .fixed_decimals(decimals)
            .update_while_editing(false),
    )
}

pub(crate) fn drag_i32_at(
    ui: &mut Ui,
    rect: Rect,
    value: &mut i32,
    suffix: &str,
    range: std::ops::RangeInclusive<i32>,
) -> Response {
    inset_field(ui.painter(), rect, FIELD);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(4.0, 1.0)))
            .layout(egui::Layout::left_to_right(Align::Center)),
    );
    child.style_mut().visuals.extreme_bg_color = FIELD;
    child.style_mut().visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    child.style_mut().visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    child.style_mut().visuals.widgets.active.bg_stroke = Stroke::NONE;
    child
        .style_mut()
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(10.5));
    child.add(egui::DragValue::new(value).range(range).suffix(suffix))
}

pub(crate) fn toggle_at(ui: &mut Ui, rect: Rect, on: &mut bool, label: &str) -> Response {
    let id = ui.next_auto_id();
    let response = ui.interact(rect, id, Sense::click());
    let checked_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 1.0, rect.center().y - 5.5),
        Vec2::splat(11.0),
    );
    let painter = ui.painter();
    if *on {
        painter.rect_filled(checked_rect, egui::CornerRadius::ZERO, ACCENT_FIELD);
        painter.text(
            checked_rect.center(),
            Align2::CENTER_CENTER,
            icons::CHECK,
            icons::font(9.0),
            TEXT,
        );
    } else {
        painter.rect_filled(checked_rect, egui::CornerRadius::ZERO, FIELD);
    };
    frame_rect(painter, checked_rect, BORDER);
    painter.text(
        Pos2::new(checked_rect.right() + 6.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(10.5),
        TEXT,
    );
    if response.clicked() {
        *on = !*on;
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

pub(crate) fn label_at(
    painter: &Painter,
    rect: Rect,
    text: &str,
    size: f32,
    color: Color32,
    align: Align2,
) {
    let font = FontId::proportional(size);
    let mut job = LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = TextWrapping::truncate_at_width(rect.width().max(1.0));
    let galley = painter.layout_job(job);
    let pos = match align {
        Align2::LEFT_CENTER => Pos2::new(rect.left(), rect.center().y - galley.size().y * 0.5),
        Align2::RIGHT_CENTER => Pos2::new(
            rect.right() - galley.size().x,
            rect.center().y - galley.size().y * 0.5,
        ),
        Align2::CENTER_CENTER => rect.center() - galley.size() * 0.5,
        _ => Pos2::new(rect.left(), rect.center().y - galley.size().y * 0.5),
    };
    painter.with_clip_rect(rect).galley(pos, galley, color);
}

pub(crate) fn mixed_icon_label(
    painter: &Painter,
    rect: Rect,
    icon: &str,
    text: &str,
    size: f32,
    color: Color32,
) {
    let icon_size = (size + 2.0).max(11.0);
    let icon_galley = painter.layout_no_wrap(icon.to_owned(), icons::font(icon_size), color);
    let text_galley = painter.layout_no_wrap(text.to_owned(), FontId::proportional(size), color);
    let gap = 5.0;
    let total = icon_galley.size().x + gap + text_galley.size().x;
    let mut x = rect.center().x - total * 0.5;
    let cy = rect.center().y;
    painter.galley(
        Pos2::new(x, cy - icon_galley.size().y * 0.5),
        icon_galley.clone(),
        color,
    );
    x += icon_galley.size().x + gap;
    painter.galley(
        Pos2::new(x, cy - text_galley.size().y * 0.5),
        text_galley,
        color,
    );
}

pub(crate) fn shadowed_device(painter: &Painter, rect: Rect) {
    painter.rect_filled(
        rect.translate(Vec2::new(2.0, 2.0)),
        egui::CornerRadius::ZERO,
        Color32::from_black_alpha(30),
    );
    painter.rect_filled(
        rect,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0xB2, 0xB2, 0xB2),
    );
    frame_rect(painter, rect, BORDER);
}
