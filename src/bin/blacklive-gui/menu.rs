use eframe::egui::{self, Align2, Color32, FontId, Id, Margin, Pos2, Rect, Sense, Stroke, Vec2};

use super::icons;
use super::theme;

#[derive(Clone, Debug)]
pub(crate) struct MenuItem {
    label: &'static str,
    shortcut: Option<&'static str>,
    checked: bool,
    enabled: bool,
    separator: bool,
}

impl MenuItem {
    pub(crate) fn action(label: &'static str) -> Self {
        Self {
            label,
            shortcut: None,
            checked: false,
            enabled: true,
            separator: false,
        }
    }

    pub(crate) fn shortcut(mut self, shortcut: &'static str) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    pub(crate) fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub(crate) fn separator() -> Self {
        Self {
            label: "",
            shortcut: None,
            checked: false,
            enabled: false,
            separator: true,
        }
    }
}

/// A small flat popup menu. Returns the index of the selected item.
///
/// This is intentionally generic: future top-bar menus should reuse it.
pub(crate) fn show(
    ctx: &egui::Context,
    id: &str,
    anchor: Rect,
    open: &mut bool,
    items: &[MenuItem],
) -> Option<usize> {
    if !*open {
        return None;
    }
    let width = 186.0;
    let item_h = 22.0;
    let separator_h = 8.0;
    let screen = ctx.content_rect();
    let max_height = items
        .iter()
        .map(|item| if item.separator { separator_h } else { item_h })
        .sum::<f32>();
    let mut pos = Pos2::new(anchor.left() - 2.0, anchor.bottom() + 3.0);
    pos.x = pos.x.clamp(
        screen.left() + 4.0,
        (screen.right() - width - 4.0).max(screen.left() + 4.0),
    );
    pos.y = pos.y.clamp(
        screen.top() + 4.0,
        (screen.bottom() - max_height - 10.0).max(screen.top() + 4.0),
    );

    let mut chosen = None;
    let window = egui::Window::new(id)
        .id(Id::unique(id))
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .fixed_pos(pos)
        .frame(
            egui::Frame::NONE
                .fill(Color32::from_rgb(0xED, 0xED, 0xED))
                .stroke(Stroke::new(1.0, theme::BORDER))
                .inner_margin(Margin::ZERO)
                .outer_margin(Margin::ZERO)
                .corner_radius(egui::CornerRadius::ZERO),
        )
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            for (index, item) in items.iter().enumerate() {
                if item.separator {
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(width, separator_h), Sense::hover());
                    ui.painter().hline(
                        (rect.left() + 7.0)..=(rect.right() - 7.0),
                        rect.center().y,
                        Stroke::new(1.0, Color32::from_rgb(0xB4, 0xB4, 0xB4)),
                    );
                    continue;
                }
                let (row, response) =
                    ui.allocate_exact_size(Vec2::new(width, item_h), Sense::click());
                if response.hovered() && item.enabled {
                    ui.painter()
                        .rect_filled(row, egui::CornerRadius::ZERO, theme::SELECTION);
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                let color = if item.enabled {
                    theme::TEXT
                } else {
                    theme::TEXT_DISABLED
                };
                let mut text_x = row.left() + 12.0;
                if item.checked {
                    ui.painter().text(
                        Pos2::new(text_x, row.center().y),
                        Align2::LEFT_CENTER,
                        icons::CHECK,
                        icons::font(11.0),
                        color,
                    );
                    text_x += 17.0;
                }
                ui.painter().text(
                    Pos2::new(text_x, row.center().y),
                    Align2::LEFT_CENTER,
                    item.label,
                    FontId::proportional(11.0),
                    color,
                );
                if let Some(shortcut) = item.shortcut {
                    ui.painter().text(
                        Pos2::new(row.right() - 12.0, row.center().y),
                        Align2::RIGHT_CENTER,
                        shortcut,
                        FontId::proportional(10.0),
                        theme::TEXT_DIM,
                    );
                }
                if response.clicked() && item.enabled {
                    chosen = Some(index);
                    *open = false;
                }
            }
        });

    if ctx.input(|input| input.pointer.any_pressed())
        && let Some(pointer) = ctx.pointer_latest_pos()
    {
        let menu_rect = window
            .as_ref()
            .map(|response| response.response.rect)
            .unwrap_or_else(|| Rect::from_min_size(pos, Vec2::new(width, max_height)));
        if !menu_rect.contains(pointer) && !anchor.contains(pointer) {
            *open = false;
        }
    }
    chosen
}
