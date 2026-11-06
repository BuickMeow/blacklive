//! Blacklive: convert a MIDI file into an Ableton Live `.als` project by
//! injecting notes into a template.
//!
//! A clean Ac Piano Upright routing template is embedded in the binary. A
//! custom template can still be supplied when the user wants a different
//! instrument, effects, or Live version. The template is the source of truth
//! for the Live version, devices, mixer state and clip schema. We only replace
//! notes and timing fields, leaving everything else XML-equivalent intact.

pub mod als;
pub mod convert;
pub mod midi;

/// Name shown in the UI and conversion results for the bundled template.
pub const DEFAULT_TEMPLATE_LABEL: &str = "内置 Ac Piano Upright 模板";
/// Clean A01 routing template: Ac Piano Upright + a routed MIDI track, dry.
pub const DEFAULT_TEMPLATE_BYTES: &[u8] = include_bytes!("../assets/default.als");

pub use als::{ClipInfo, ClipLocation, TemplateReport, TrackInfo, TrackKind};
pub use convert::{
    ConversionResult, ConvertOptions, GroupResult, TrackResult, convert_midi_to_als,
    convert_midi_to_als_with_default_template, inspect_template, parse_channel_filter,
};
pub use midi::{ControllerEvent, MidiStats, Note, ParsedMidi, parse_midi_file};

/// Inspect the embedded default template without touching the filesystem.
pub fn inspect_default_template() -> anyhow::Result<TemplateReport> {
    als::inspect_template_bytes(DEFAULT_TEMPLATE_LABEL, DEFAULT_TEMPLATE_BYTES)
}

/// Dump the embedded template's XML for debugging.
pub fn dump_default_template_xml(output: &std::path::Path) -> anyhow::Result<()> {
    als::dump_als_bytes_xml(DEFAULT_TEMPLATE_LABEL, DEFAULT_TEMPLATE_BYTES, output)
}

/// Format an Ableton XML number without trailing zeros or scientific notation.
///
/// Ableton writes beats as decimal strings; scientific notation (`1e-7`) is not
/// used in `.als` files and is best avoided even though XML would accept it.
pub fn format_num(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let text = format!("{value:.12}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" || text.is_empty() {
        "0".to_string()
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::format_num;

    #[test]
    fn formats_like_ableton() {
        assert_eq!(format_num(0.0), "0");
        assert_eq!(format_num(1.0), "1");
        assert_eq!(format_num(0.25), "0.25");
        assert_eq!(format_num(1.0 / 480.0), "0.002083333333");
        assert_eq!(format_num(-0.0000000000001), "0");
        assert_eq!(format_num(120.0), "120");
    }
}
