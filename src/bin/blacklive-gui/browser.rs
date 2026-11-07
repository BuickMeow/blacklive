use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2,
};

use super::App;
use super::actions::UiActions;
use super::icons;
use super::project::ProjectView;
use super::theme;

const RAIL_W: f32 = 105.0;
const SEARCH_H: f32 = 24.0;
const TAG_H: f32 = 20.0;

pub(crate) fn show(ui: &mut Ui, app: &mut App, project: &ProjectView, actions: &mut UiActions) {
    let rect = ui.max_rect();
    let painter = ui.painter().clone();
    painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE);

    let search_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), SEARCH_H));
    painter.rect_filled(search_rect, egui::CornerRadius::ZERO, theme::SURFACE);
    painter.hline(
        search_rect.x_range(),
        search_rect.bottom() - 0.5,
        Stroke::new(1.0, theme::BORDER_SOFT),
    );

    let top = rect.top() + 3.0;
    let h = 18.0;
    let back = Rect::from_min_size(Pos2::new(rect.left() + 7.0, top), Vec2::new(18.0, h));
    theme::icon_button(ui, back, icons::CHEVRON_LEFT, false, false);
    let forward = Rect::from_min_size(Pos2::new(rect.left() + 27.0, top), Vec2::new(18.0, h));
    theme::icon_button(ui, forward, icons::CHEVRON_RIGHT, false, false);

    let search_field = Rect::from_min_size(
        Pos2::new(rect.left() + 50.0, top),
        Vec2::new(rect.width() - 50.0 - 58.0, h),
    );
    theme::text_edit_at(
        ui,
        search_field,
        &mut app.browser_search,
        "搜索 (Cmd + F)",
        10.5,
        egui::Align::LEFT,
    );
    let filter = Rect::from_min_size(
        Pos2::new(search_field.right() + 4.0, top),
        Vec2::new(26.0, h),
    );
    if theme::icon_button(ui, filter, icons::FILTER_LIST, app.browser_filters, true).clicked() {
        app.browser_filters = !app.browser_filters;
    }
    let filter_drop = Rect::from_min_size(Pos2::new(filter.right() + 2.0, top), Vec2::new(16.0, h));
    theme::icon_button(ui, filter_drop, icons::ARROW_DROP_DOWN, false, true);

    let body = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + SEARCH_H), rect.max);
    let rail = Rect::from_min_max(body.min, Pos2::new(body.left() + RAIL_W, body.bottom()));
    let tree = Rect::from_min_max(Pos2::new(rail.right(), body.top()), body.max);
    linguist_rail(ui, app, rail);
    tree_content(ui, app, project, tree, actions);

    painter.vline(
        rail.right() - 0.5,
        body.y_range(),
        Stroke::new(1.0, theme::BORDER_SOFT),
    );
    painter.hline(
        Rect::from_min_max(rail.min, Pos2::new(rail.right(), rail.top())).x_range(),
        rail.top() + 0.5,
        Stroke::new(1.0, theme::PANEL_HI),
    );
}

struct RailItem {
    label: &'static str,
    icon: &'static str,
}

fn linguist_rail(ui: &mut Ui, app: &mut App, rect: Rect) {
    let mut y = rect.top() + 2.0;
    let row_h = 19.0;

    // Top "collection" block from Live.
    theme::label_at(
        ui.painter(),
        Rect::from_min_size(
            Pos2::new(rect.left() + 10.0, y),
            Vec2::new(rect.width() - 16.0, 16.0),
        ),
        "收藏夹",
        10.0,
        theme::TEXT_DIM,
        Align2::LEFT_CENTER,
    );
    y += 18.0;
    draw_favorite_row(ui, rect, y, row_h);
    y += row_h + 4.0;

    for (header, items) in [
        (
            "音色库",
            vec![
                RailItem {
                    label: "全部",
                    icon: icons::LIBRARY_MUSIC,
                },
                RailItem {
                    label: "声音",
                    icon: icons::MUSIC_NOTE,
                },
                RailItem {
                    label: "鼓",
                    icon: icons::ALBUM,
                },
                RailItem {
                    label: "乐器",
                    icon: icons::PIANO,
                },
                RailItem {
                    label: "音频效果器",
                    icon: icons::GRAPHIC_EQ,
                },
                RailItem {
                    label: "MIDI 效果器",
                    icon: icons::TUNE,
                },
                RailItem {
                    label: "调制器",
                    icon: icons::WAVES,
                },
                RailItem {
                    label: "Max for Live",
                    icon: icons::GRID_VIEW,
                },
                RailItem {
                    label: "插件",
                    icon: icons::SETTINGS,
                },
                RailItem {
                    label: "Clip",
                    icon: icons::INSERT_DRIVE_FILE,
                },
                RailItem {
                    label: "采样",
                    icon: icons::AUDIOTRACK,
                },
                RailItem {
                    label: "律动",
                    icon: icons::SPEED,
                },
                RailItem {
                    label: "调律",
                    icon: icons::PIANO,
                },
                RailItem {
                    label: "模板",
                    icon: icons::ALBUM,
                },
            ],
        ),
        (
            "位置",
            vec![
                RailItem {
                    label: "Packs",
                    icon: icons::FOLDER_OPEN,
                },
                RailItem {
                    label: "Splice",
                    icon: icons::WAVES,
                },
                RailItem {
                    label: "Cloud",
                    icon: icons::WAVES,
                },
                RailItem {
                    label: "Push",
                    icon: icons::GRID_VIEW,
                },
                RailItem {
                    label: "用户库",
                    icon: icons::FOLDER_OPEN,
                },
                RailItem {
                    label: "当前工程",
                    icon: icons::FOLDER_OPEN,
                },
                RailItem {
                    label: "MIDI",
                    icon: icons::MUSIC_NOTE,
                },
                RailItem {
                    label: "添加文件夹…",
                    icon: icons::ADD,
                },
            ],
        ),
    ] {
        theme::label_at(
            ui.painter(),
            Rect::from_min_size(
                Pos2::new(rect.left() + 10.0, y),
                Vec2::new(rect.width() - 16.0, 16.0),
            ),
            header,
            10.0,
            theme::TEXT_DIM,
            Align2::LEFT_CENTER,
        );
        y += 17.0;
        for item in items {
            if y + row_h > rect.bottom() {
                return;
            }
            let row = Rect::from_min_size(
                Pos2::new(rect.left() + 4.0, y),
                Vec2::new(rect.width() - 8.0, row_h - 1.0),
            );
            let selected = app.browser_section == item.label;
            let response = ui.interact(
                row,
                ui.make_persistent_id(("rail", item.label)),
                Sense::click(),
            );
            if selected {
                ui.painter()
                    .rect_filled(row, egui::CornerRadius::ZERO, theme::SELECTION);
            } else if response.hovered() {
                ui.painter().rect_filled(
                    row,
                    egui::CornerRadius::ZERO,
                    Color32::from_rgb(0xB8, 0xB8, 0xB8),
                );
            }
            let color = if selected {
                theme::TEXT
            } else {
                theme::TEXT_DIM
            };
            ui.painter().text(
                Pos2::new(row.left() + 7.0, row.center().y),
                Align2::LEFT_CENTER,
                item.icon,
                icons::font(12.5),
                color,
            );
            theme::label_at(
                ui.painter(),
                Rect::from_min_max(
                    Pos2::new(row.left() + 25.0, row.top()),
                    Pos2::new(row.right() - 2.0, row.bottom()),
                ),
                item.label,
                10.5,
                color,
                Align2::LEFT_CENTER,
            );
            if response.clicked() {
                app.browser_section = item.label.to_string();
                app.set_status(format!("浏览：{}", item.label));
            }
            if response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            y += row_h;
        }
        y += 4.0;
    }
}

fn draw_favorite_row(ui: &mut Ui, rect: Rect, y: f32, row_h: f32) {
    let row = Rect::from_min_size(
        Pos2::new(rect.left() + 4.0, y),
        Vec2::new(rect.width() - 8.0, row_h - 1.0),
    );
    ui.painter().rect_filled(
        Rect::from_min_size(
            Pos2::new(row.left() + 7.0, row.center().y - 3.5),
            Vec2::splat(7.0),
        ),
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0xD8, 0x3A, 0x2E),
    );
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(row.left() + 20.0, row.top()),
            Pos2::new(row.right() - 2.0, row.bottom()),
        ),
        "お気に入り",
        10.5,
        theme::TEXT,
        Align2::LEFT_CENTER,
    );
}

fn tree_content(
    ui: &mut Ui,
    app: &mut App,
    project: &ProjectView,
    rect: Rect,
    actions: &mut UiActions,
) {
    let mut y = rect.top() + 2.0;
    theme::label_at(
        ui.painter(),
        Rect::from_min_size(
            Pos2::new(rect.left() + 10.0, y),
            Vec2::new(rect.width() - 20.0, 15.0),
        ),
        "Content",
        10.5,
        theme::TEXT,
        Align2::LEFT_CENTER,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 57.0, y + 7.5),
        Align2::LEFT_CENTER,
        icons::ARROW_DROP_DOWN,
        icons::font(12.0),
        theme::TEXT_DIM,
    );
    y += 16.0;

    if app.browser_filters {
        y = tag_rows(ui, app, rect, y);
    }

    y += 2.0;
    let header_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), 18.0));
    ui.painter()
        .rect_filled(header_rect, egui::CornerRadius::ZERO, theme::SURFACE_LIGHT);
    ui.painter().hline(
        header_rect.x_range(),
        header_rect.bottom() - 0.5,
        Stroke::new(1.0, theme::BORDER_SOFT),
    );
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(header_rect.left() + 10.0, header_rect.top()),
            Pos2::new(header_rect.right() - 30.0, header_rect.bottom()),
        ),
        "名称",
        10.0,
        theme::TEXT_DIM,
        Align2::LEFT_CENTER,
    );
    ui.painter().text(
        Pos2::new(header_rect.right() - 30.0, header_rect.center().y),
        Align2::CENTER_CENTER,
        icons::EXPAND_LESS,
        icons::font(11.0),
        theme::TEXT_DIM,
    );
    ui.painter().text(
        Pos2::new(header_rect.right() - 12.0, header_rect.center().y),
        Align2::CENTER_CENTER,
        icons::MORE_HORIZ,
        icons::font(12.0),
        theme::TEXT_DIM,
    );
    y += 19.0;

    let list_rect = Rect::from_min_max(Pos2::new(rect.left(), y), rect.max);
    ui.painter()
        .rect_filled(list_rect, egui::CornerRadius::ZERO, theme::SURFACE);
    ui.painter().vline(
        list_rect.left() + 0.5,
        list_rect.y_range(),
        Stroke::new(1.0, theme::BORDER_SOFT),
    );

    if app.browser_section != "当前工程" && app.browser_section != "全部" {
        theme::label_at(
            ui.painter(),
            Rect::from_min_size(
                Pos2::new(list_rect.left() + 12.0, list_rect.top() + 16.0),
                Vec2::new(list_rect.width() - 24.0, 18.0),
            ),
            "此分类在 Blacklive 中暂未接入内容。",
            10.0,
            theme::TEXT_DIM,
            Align2::LEFT_CENTER,
        );
        return;
    }

    let row_h = 18.0;
    let mut row_y = list_rect.top() + 2.0;
    let mut row_index = 0usize;

    let midi_name = file_name(&app.midi_path).unwrap_or_else(|| "选择 MIDI…".to_string());
    let template_name = if app.template_path.trim().is_empty() {
        "内置 Ac Piano Upright".to_string()
    } else {
        file_name(&app.template_path).unwrap_or_else(|| app.template_path.clone())
    };
    let output_name = file_name(&app.output_path).unwrap_or_else(|| "与 MIDI 同名".to_string());

    let rows: Vec<RowSpec> = vec![
        RowSpec::action("MIDI 文件", icons::MUSIC_NOTE, midi_name, "midi"),
        RowSpec::action("模板", icons::ALBUM, template_name, "template"),
        RowSpec::action("输出 .als", icons::SAVE, output_name, "output"),
        RowSpec::separator(),
        RowSpec::action("解析 MIDI", icons::TUNE, "分析".to_string(), "analyze_midi"),
        RowSpec::action(
            "分析模板",
            icons::INFO,
            "检查".to_string(),
            "analyze_template",
        ),
        RowSpec::action(
            "导出模板 XML",
            icons::DOWNLOAD,
            "XML".to_string(),
            "dump_xml",
        ),
        RowSpec::action(
            "使用内置模板",
            icons::REFRESH,
            "重置".to_string(),
            "clear_template",
        ),
        RowSpec::separator(),
        RowSpec::info("BPM", format!("{:.2}", project.bpm)),
        RowSpec::info("音符", project.total_notes.to_string()),
        RowSpec::info("通道", project.tracks.len().to_string()),
        RowSpec::info(
            "时长",
            format_duration(project.duration_beats, project.time_signature, project.bpm),
        ),
    ];

    let needle = app.browser_search.trim().to_lowercase();
    for spec in rows {
        if !needle.is_empty()
            && spec.kind != RowKind::Separator
            && !spec.label.to_lowercase().contains(&needle)
            && !spec.value.to_lowercase().contains(&needle)
        {
            continue;
        }
        if spec.kind == RowKind::Separator {
            let sep = Rect::from_min_size(
                Pos2::new(list_rect.left() + 8.0, row_y + 2.0),
                Vec2::new(list_rect.width() - 16.0, 1.0),
            );
            ui.painter().hline(
                sep.x_range(),
                sep.center().y,
                Stroke::new(1.0, theme::BORDER_SOFT),
            );
            row_y += 4.0;
            continue;
        }
        if row_y + row_h > list_rect.bottom() {
            break;
        }
        let row = Rect::from_min_size(
            Pos2::new(list_rect.left() + 2.0, row_y),
            Vec2::new(list_rect.width() - 4.0, row_h - 1.0),
        );
        row_index += 1;
        draw_row(ui, app, spec, row, row_index, actions);
        row_y += row_h;
    }

    if !project.placeholder && !project.tracks.is_empty() {
        row_y += 4.0;
        if row_y + 16.0 < list_rect.bottom() {
            let section = Rect::from_min_size(
                Pos2::new(list_rect.left(), row_y),
                Vec2::new(list_rect.width(), 16.0),
            );
            ui.painter()
                .rect_filled(section, egui::CornerRadius::ZERO, theme::SURFACE_LIGHT);
            theme::label_at(
                ui.painter(),
                section.shrink2(Vec2::new(10.0, 0.0)),
                "通道",
                9.5,
                theme::TEXT_DIM,
                Align2::LEFT_CENTER,
            );
            theme::label_at(
                ui.painter(),
                section.shrink2(Vec2::new(10.0, 0.0)),
                &format!("{} 轨", project.tracks.len()),
                9.0,
                theme::TEXT_DIM,
                Align2::RIGHT_CENTER,
            );
            row_y += 17.0;
        }
        for (index, track) in project.tracks.iter().enumerate() {
            if row_y + row_h > list_rect.bottom() {
                break;
            }
            draw_channel_row(ui, app, track, index, list_rect, row_y);
            row_y += row_h;
        }
    }

    // A soft empty-state hint at the bottom of the browser.
    if app.midi_path.trim().is_empty() {
        theme::label_at(
            ui.painter(),
            Rect::from_min_size(
                Pos2::new(list_rect.left() + 12.0, list_rect.bottom() - 28.0),
                Vec2::new(list_rect.width() - 24.0, 18.0),
            ),
            "把 MIDI 拖进窗口，或点击上方「选择 MIDI」。",
            9.5,
            theme::TEXT_DIM,
            Align2::LEFT_CENTER,
        );
    }
}

fn draw_channel_row(
    ui: &mut Ui,
    app: &mut App,
    track: &super::project::TrackView,
    index: usize,
    list_rect: Rect,
    y: f32,
) {
    let row = Rect::from_min_size(
        Pos2::new(list_rect.left() + 2.0, y),
        Vec2::new(list_rect.width() - 4.0, 17.0),
    );
    let selected = app.selected_track == Some(index);
    let response = ui.interact(
        row,
        ui.make_persistent_id(("browser_channel", index)),
        Sense::click(),
    );
    if selected {
        ui.painter()
            .rect_filled(row, egui::CornerRadius::ZERO, theme::SELECTION);
    } else if response.hovered() {
        ui.painter().rect_filled(
            row,
            egui::CornerRadius::ZERO,
            Color32::from_rgb(0xC9, 0xC9, 0xC9),
        );
    }
    let swatch = Rect::from_min_size(
        Pos2::new(row.left() + 18.0, row.center().y - 5.0),
        Vec2::new(18.0, 10.0),
    );
    ui.painter()
        .rect_filled(swatch, egui::CornerRadius::ZERO, track.color);
    theme::frame_rect(ui.painter(), swatch, theme::BORDER_SOFT);
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(swatch.right() + 7.0, row.top()),
            Pos2::new(row.right() - 56.0, row.bottom()),
        ),
        &track.name,
        10.0,
        theme::TEXT,
        Align2::LEFT_CENTER,
    );
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(row.right() - 54.0, row.top()),
            Pos2::new(row.right() - 6.0, row.bottom()),
        ),
        &format!("{} 音符", track.note_count),
        8.8,
        theme::TEXT_DIM,
        Align2::RIGHT_CENTER,
    );
    if response.clicked() {
        app.selected_track = Some(index);
        app.set_status(format!("选中 {}", track.name));
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RowKind {
    Action,
    Info,
    Separator,
}

struct RowSpec {
    kind: RowKind,
    label: String,
    value: String,
    icon: &'static str,
    action: &'static str,
}

impl RowSpec {
    fn action(label: &str, icon: &'static str, value: String, action: &'static str) -> Self {
        Self {
            kind: RowKind::Action,
            label: label.to_string(),
            value,
            icon,
            action,
        }
    }

    fn info(label: &str, value: String) -> Self {
        Self {
            kind: RowKind::Info,
            label: label.to_string(),
            value,
            icon: icons::INFO,
            action: "",
        }
    }

    fn separator() -> Self {
        Self {
            kind: RowKind::Separator,
            label: String::new(),
            value: String::new(),
            icon: icons::INFO,
            action: "",
        }
    }
}

fn draw_row(
    ui: &mut Ui,
    app: &mut App,
    spec: RowSpec,
    rect: Rect,
    row_index: usize,
    actions: &mut UiActions,
) {
    let id = ui.make_persistent_id(("browser_row", row_index, spec.label.as_str()));
    let response = ui.interact(rect, id, Sense::click());
    if response.hovered() && spec.kind == RowKind::Action {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::ZERO,
            Color32::from_rgb(0xC9, 0xC9, 0xC9),
        );
    }
    if spec.kind == RowKind::Action {
        ui.painter().text(
            Pos2::new(rect.left() + 4.0, rect.center().y),
            Align2::LEFT_CENTER,
            icons::ARROW_RIGHT,
            icons::font(10.0),
            theme::TEXT_DIM,
        );
    }
    let icon_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 18.0, rect.center().y - 7.0),
        Vec2::new(24.0, 14.0),
    );
    ui.painter().rect_filled(
        icon_rect,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(0xC8, 0xC8, 0xC8),
    );
    theme::frame_rect(ui.painter(), icon_rect, Color32::from_rgb(0x6B, 0x6B, 0x6B));
    ui.painter().text(
        icon_rect.center(),
        Align2::CENTER_CENTER,
        spec.icon,
        icons::font(11.0),
        theme::TEXT,
    );
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(icon_rect.right() + 7.0, rect.top()),
            Pos2::new(rect.right() - 120.0, rect.bottom()),
        ),
        &spec.label,
        10.5,
        theme::TEXT,
        Align2::LEFT_CENTER,
    );
    theme::label_at(
        ui.painter(),
        Rect::from_min_max(
            Pos2::new(rect.right() - 118.0, rect.top()),
            Pos2::new(rect.right() - 6.0, rect.bottom()),
        ),
        &spec.value,
        9.5,
        if spec.kind == RowKind::Info {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        },
        Align2::RIGHT_CENTER,
    );
    if spec.kind == RowKind::Action && response.clicked() {
        match spec.action {
            "midi" => actions.choose_midi = true,
            "template" => actions.choose_template = true,
            "output" => actions.choose_output = true,
            "analyze_midi" => actions.analyze_midi = true,
            "analyze_template" => actions.analyze_template = true,
            "dump_xml" => actions.dump_xml = true,
            "clear_template" => actions.clear_template = true,
            _ => {}
        }
        app.set_status(format!("{}…", spec.label));
    }
    if response.hovered() && spec.kind == RowKind::Action {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
}

fn tag_rows(ui: &mut Ui, app: &mut App, rect: Rect, mut y: f32) -> f32 {
    let tags: [&[&str]; 2] = [
        &["全部", "文件", "MIDI", "模板", "输出"],
        &["分析", "转换", "设置", "日志"],
    ];
    for row in tags {
        let mut x = rect.left() + 10.0;
        for tag in row {
            let font = FontId::proportional(9.5);
            let galley = ui
                .painter()
                .layout_no_wrap(tag.to_string(), font, theme::TEXT);
            let w = galley.size().x + 12.0;
            if x + w > rect.right() - 4.0 {
                break;
            }
            let chip = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, TAG_H - 3.0));
            let active = app.browser_tag == *tag;
            let response = ui.interact(chip, ui.make_persistent_id(("tag", *tag)), Sense::click());
            let fill = if active {
                theme::ACCENT
            } else if response.hovered() {
                Color32::from_rgb(0xC4, 0xC4, 0xC4)
            } else {
                Color32::from_rgb(0xB6, 0xB6, 0xB6)
            };
            ui.painter()
                .rect_filled(chip, egui::CornerRadius::ZERO, fill);
            theme::frame_rect(ui.painter(), chip, theme::BORDER_SOFT);
            ui.painter().text(
                chip.center(),
                Align2::CENTER_CENTER,
                tag,
                FontId::proportional(9.5),
                theme::TEXT,
            );
            if response.clicked() {
                app.browser_tag = (*tag).to_string();
            }
            x += w + 3.0;
        }
        y += TAG_H - 1.0;
    }
    y + 2.0
}

fn file_name(path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    Some(
        std::path::Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string()),
    )
}

fn format_duration(beats: f64, time_signature: (u8, u16), bpm: f64) -> String {
    let beats_per_bar = time_signature.0.max(1) as f64 * 4.0 / time_signature.1.max(1) as f64;
    let bars = (beats / beats_per_bar).max(0.0);
    let minutes = if bpm > 0.0 { beats / bpm } else { 0.0 };
    if minutes >= 1.0 {
        let whole = minutes.floor() as u64;
        let seconds = ((minutes - whole as f64) * 60.0).round() as u64;
        format!("{:.2} 小节 / {}:{:02}", bars, whole, seconds)
    } else {
        format!("{:.2} 小节 / {:.1} 秒", bars, minutes * 60.0)
    }
}
