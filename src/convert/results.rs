use std::path::PathBuf;

use crate::midi::MidiStats;

#[derive(Clone, Debug)]
pub struct TrackResult {
    pub name: String,
    pub source_track: u16,
    pub note_count: usize,
    pub clip_length_beats: f64,
}

#[derive(Clone, Debug)]
pub struct GroupResult {
    pub channel: u8,
    pub name: String,
    pub tracks: Vec<TrackResult>,
}

#[derive(Clone, Debug)]
pub struct ConversionResult {
    pub output_path: PathBuf,
    pub template_label: String,
    pub midi_path: PathBuf,
    pub template_clip: String,
    pub bpm: f64,
    pub tempo_automation: bool,
    pub clip_start_beats: f64,
    pub clip_length_beats: f64,
    pub source_note_count: usize,
    pub used_note_count: usize,
    pub dropped_by_channel: usize,
    pub dropped_by_transpose: usize,
    pub clamped_short_notes: usize,
    pub trimmed_overlaps: usize,
    pub groups: Vec<GroupResult>,
    pub midi_stats: MidiStats,
    pub warnings: Vec<String>,
}
