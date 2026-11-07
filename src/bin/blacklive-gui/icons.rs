#![allow(dead_code)]

use eframe::egui::{self, FontFamily, FontId};

pub(crate) const MATERIAL_FAMILY: &str = "material";

pub(crate) const ADD: &str = "\u{e145}";
pub(crate) const ALBUM: &str = "\u{e019}";
pub(crate) const ARROW_BACK: &str = "\u{e5c4}";
pub(crate) const ARROW_DROP_DOWN: &str = "\u{e5c5}";
pub(crate) const ARROW_FORWARD: &str = "\u{e5c8}";
pub(crate) const ARROW_RIGHT: &str = "\u{e5df}";
pub(crate) const AUDIOTRACK: &str = "\u{e3a1}";
pub(crate) const CANCEL: &str = "\u{e5c9}";
pub(crate) const CHECK: &str = "\u{e5ca}";
pub(crate) const CHECK_CIRCLE: &str = "\u{e86c}";
pub(crate) const CHEVRON_LEFT: &str = "\u{e5cb}";
pub(crate) const CHEVRON_RIGHT: &str = "\u{e5cc}";
pub(crate) const CLOSE: &str = "\u{e5cd}";
pub(crate) const DELETE: &str = "\u{e872}";
pub(crate) const DOWNLOAD: &str = "\u{f090}";
pub(crate) const DRAG_INDICATOR: &str = "\u{e945}";
pub(crate) const EQUALIZER: &str = "\u{e01d}";
pub(crate) const ERROR: &str = "\u{e000}";
pub(crate) const EXPAND_LESS: &str = "\u{e5ce}";
pub(crate) const EXPAND_MORE: &str = "\u{e5cf}";
pub(crate) const FAST_FORWARD: &str = "\u{e01f}";
pub(crate) const FAST_REWIND: &str = "\u{e020}";
pub(crate) const FIBER_MANUAL_RECORD: &str = "\u{e061}";
pub(crate) const FILTER_LIST: &str = "\u{e152}";
pub(crate) const FIRST_PAGE: &str = "\u{e5dc}";
pub(crate) const FOLDER_OPEN: &str = "\u{e2c8}";
pub(crate) const GRAPHIC_EQ: &str = "\u{e1b8}";
pub(crate) const GRID_VIEW: &str = "\u{e9b0}";
pub(crate) const HEADPHONES: &str = "\u{f01f}";
pub(crate) const HELP: &str = "\u{e887}";
pub(crate) const INFO: &str = "\u{e88e}";
pub(crate) const INSERT_DRIVE_FILE: &str = "\u{e24d}";
pub(crate) const LAST_PAGE: &str = "\u{e5dd}";
pub(crate) const LIBRARY_MUSIC: &str = "\u{e030}";
pub(crate) const LINK: &str = "\u{e157}";
pub(crate) const LOCK: &str = "\u{e897}";
pub(crate) const LOOP: &str = "\u{e028}";
pub(crate) const MENU: &str = "\u{e5d2}";
pub(crate) const MIC: &str = "\u{e029}";
pub(crate) const MORE_HORIZ: &str = "\u{e5d3}";
pub(crate) const MUSIC_NOTE: &str = "\u{e405}";
pub(crate) const OPEN_IN_NEW: &str = "\u{e89e}";
pub(crate) const PAUSE: &str = "\u{e034}";
pub(crate) const PIANO: &str = "\u{e521}";
pub(crate) const PLAY_ARROW: &str = "\u{e037}";
pub(crate) const PLAY_CIRCLE: &str = "\u{e1c4}";
pub(crate) const REFRESH: &str = "\u{e5d5}";
pub(crate) const REMOVE: &str = "\u{e15b}";
pub(crate) const REPLAY: &str = "\u{e042}";
pub(crate) const SAVE: &str = "\u{e161}";
pub(crate) const SEARCH: &str = "\u{e8b6}";
pub(crate) const SETTINGS: &str = "\u{e8b8}";
pub(crate) const SKIP_NEXT: &str = "\u{e044}";
pub(crate) const SKIP_PREVIOUS: &str = "\u{e045}";
pub(crate) const SPEED: &str = "\u{e9e4}";
pub(crate) const STOP: &str = "\u{e047}";
pub(crate) const TIMER: &str = "\u{e425}";
pub(crate) const TUNE: &str = "\u{e429}";
pub(crate) const UNDO: &str = "\u{e166}";
pub(crate) const UPLOAD: &str = "\u{f09b}";
pub(crate) const VIEW_AGENDA: &str = "\u{e8e9}";
pub(crate) const VISIBILITY: &str = "\u{e8f4}";
pub(crate) const VOLUME_UP: &str = "\u{e050}";
pub(crate) const WARNING: &str = "\u{e002}";
pub(crate) const WAVES: &str = "\u{e176}";

pub(crate) fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(MATERIAL_FAMILY.into()))
}

pub(crate) fn material_font_data() -> egui::FontData {
    egui::FontData::from_static(include_bytes!("../../../assets/MaterialIcons-Regular.ttf"))
}
