use std::path::Path;

use blacklive::{ConversionResult, MidiStats};
use eframe::egui::Color32;

use super::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViewMode {
    Arrangement,
    Session,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrackKind {
    Midi,
    Audio,
}

#[derive(Clone, Debug)]
pub(crate) struct ClipView {
    pub(crate) name: String,
    pub(crate) start_beats: f64,
    pub(crate) length_beats: f64,
    pub(crate) color: Color32,
}

#[derive(Clone, Debug)]
pub(crate) struct TrackView {
    pub(crate) name: String,
    pub(crate) kind: TrackKind,
    pub(crate) color: Color32,
    #[allow(dead_code)]
    pub(crate) note_count: usize,
    pub(crate) clips: Vec<ClipView>,
    pub(crate) armed: bool,
    pub(crate) muted: bool,
    pub(crate) solo: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ReturnView {
    pub(crate) name: String,
    pub(crate) color: Color32,
    pub(crate) letter: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ProjectView {
    pub(crate) title: String,
    pub(crate) tracks: Vec<TrackView>,
    pub(crate) returns: Vec<ReturnView>,
    pub(crate) bpm: f64,
    pub(crate) time_signature: (u8, u16),
    pub(crate) duration_beats: f64,
    pub(crate) total_notes: usize,
    pub(crate) placeholder: bool,
}

pub(crate) fn placeholder_tracks() -> Vec<TrackView> {
    [
        ("1 MIDI", TrackKind::Midi, theme::TRACK_COLORS[0]),
        ("2 MIDI", TrackKind::Midi, theme::TRACK_COLORS[1]),
        ("3 Audio", TrackKind::Audio, theme::TRACK_COLORS[2]),
        ("4 Audio", TrackKind::Audio, theme::TRACK_COLORS[3]),
    ]
    .into_iter()
    .map(|(name, kind, color)| TrackView {
        name: name.to_string(),
        kind,
        color,
        note_count: 0,
        clips: Vec::new(),
        armed: false,
        muted: false,
        solo: false,
    })
    .collect()
}

pub(crate) fn returns() -> Vec<ReturnView> {
    vec![
        ReturnView {
            name: "A Reverb".to_string(),
            color: theme::RETURN_A,
            letter: "A".to_string(),
        },
        ReturnView {
            name: "B Delay".to_string(),
            color: theme::RETURN_B,
            letter: "B".to_string(),
        },
        ReturnView {
            name: "Main".to_string(),
            color: theme::MAIN,
            letter: "Main".to_string(),
        },
    ]
}

pub(crate) fn project_title(midi_path: &str) -> String {
    if midi_path.trim().is_empty() {
        return "无标题".to_string();
    }
    Path::new(midi_path.trim())
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "无标题".to_string())
}

pub(crate) fn view_from(
    midi_path: &str,
    stats: Option<&MidiStats>,
    result: Option<&ConversionResult>,
    bpm_override: Option<f64>,
    start_beats: f64,
) -> ProjectView {
    let title = project_title(midi_path);
    let returns = returns();
    let Some(stats) = stats else {
        let bpm = bpm_override.or(result.map(|r| r.bpm)).unwrap_or(120.0);
        return ProjectView {
            title,
            tracks: placeholder_tracks(),
            returns,
            bpm,
            time_signature: (4, 4),
            duration_beats: 0.0,
            total_notes: 0,
            placeholder: true,
        };
    };

    let mut channels: Vec<u8> = stats
        .notes_per_channel
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(channel, _)| *channel)
        .collect();
    for channel in stats.controllers.keys() {
        if !channels.contains(channel) {
            channels.push(*channel);
        }
    }
    for channel in stats.pitch_bends.keys() {
        if !channels.contains(channel) {
            channels.push(*channel);
        }
    }
    channels.sort_unstable();

    let duration_beats = stats.duration_beats.max(1.0);
    let mut tracks = Vec::new();
    for (index, channel) in channels.iter().enumerate() {
        let note_count = stats
            .notes_per_channel
            .get(channel)
            .copied()
            .unwrap_or_default();
        let color = theme::TRACK_COLORS[index % theme::TRACK_COLORS.len()];
        let clip_length = result
            .and_then(|result| result.groups.iter().find(|group| group.channel == *channel))
            .map(|group| {
                group
                    .tracks
                    .iter()
                    .map(|track| track.clip_length_beats)
                    .fold(0.0_f64, f64::max)
            })
            .filter(|length| *length > 0.0)
            .unwrap_or(duration_beats);
        let clips = if note_count > 0 {
            vec![ClipView {
                name: format!("A{:02}", channel + 1),
                start_beats,
                length_beats: clip_length,
                color,
            }]
        } else {
            Vec::new()
        };
        tracks.push(TrackView {
            name: format!("A{:02}", channel + 1),
            kind: TrackKind::Midi,
            color,
            note_count,
            clips,
            armed: false,
            muted: false,
            solo: false,
        });
    }

    if tracks.is_empty() {
        tracks = placeholder_tracks();
    }

    let time_signature = stats
        .time_signatures
        .first()
        .map(|sig| (sig.numerator, sig.denominator))
        .unwrap_or((4, 4));
    let bpm = bpm_override
        .or(stats.initial_bpm)
        .or(result.map(|r| r.bpm))
        .unwrap_or(120.0);

    ProjectView {
        title,
        tracks,
        returns,
        bpm,
        time_signature,
        duration_beats,
        total_notes: stats.note_count,
        placeholder: false,
    }
}

/// Format an absolute beat position as Live's `bar.beat.sixteenth` display.
pub(crate) fn position_label(beats: f64, time_signature: (u8, u16)) -> String {
    let numerator = time_signature.0.max(1) as f64;
    let denominator = time_signature.1.max(1) as f64;
    let beat_unit = 4.0 / denominator;
    let bar_len = numerator * beat_unit;
    let beats = beats.max(0.0);
    let bar = (beats / bar_len).floor() as i64 + 1;
    let in_bar = beats % bar_len;
    let beat = (in_bar / beat_unit).floor() as i64 + 1;
    let frac = (in_bar % beat_unit) / beat_unit;
    let sixteenth = (frac * 4.0).floor() as i64 + 1;
    format!("{bar}. {beat}. {sixteenth}")
}
