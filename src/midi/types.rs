use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Debug)]
pub struct Note {
    /// MIDI key number, 0..=127.
    pub key: u8,
    /// Start position in quarter-note beats.
    pub start: f64,
    /// Duration in quarter-note beats.
    pub duration: f64,
    /// MIDI velocity, 1..=127.
    pub velocity: f64,
    /// MIDI note-off velocity, normally 64.
    pub off_velocity: f64,
    /// Source MIDI channel, 0..=15.
    pub channel: u8,
    /// Source MIDI track index in the file.
    pub source_track: u16,
}

#[derive(Clone, Debug)]
pub struct ControllerEvent {
    /// Absolute MIDI tick.
    pub tick: u64,
    /// Raw 0..=127 controller value.
    pub value: u8,
}

#[derive(Clone, Debug)]
pub struct PitchBendEvent {
    /// Absolute MIDI tick.
    pub tick: u64,
    /// Raw 14-bit value, 0..=16383; 8192 is center.
    pub value: u16,
}

/// RPN 0,0 (Pitch Bend Sensitivity) change, in semitones.
#[derive(Clone, Debug)]
pub struct PitchBendRangeEvent {
    /// Absolute MIDI tick.
    pub tick: u64,
    /// Range in semitones; fractional part comes from Data Entry LSB cents.
    pub semitones: f64,
}

#[derive(Clone, Debug)]
pub struct TimeSignatureInfo {
    pub tick: u64,
    pub numerator: u8,
    pub denominator: u16,
}

#[derive(Clone, Debug)]
pub struct TempoEventInfo {
    /// Absolute MIDI tick.
    pub tick: u64,
    pub bpm: f64,
}

#[derive(Clone, Debug)]
pub struct MidiStats {
    pub format: String,
    pub ppq: u16,
    pub track_count: usize,
    pub track_names: Vec<String>,
    /// Track names by source track index; missing/empty names are omitted.
    pub track_names_by_index: BTreeMap<u16, String>,
    /// source track -> channels that had any MIDI message.
    pub track_channels: BTreeMap<u16, BTreeSet<u8>>,
    /// channel -> controller number -> absolute-tick events.
    pub controllers: BTreeMap<u8, BTreeMap<u8, Vec<ControllerEvent>>>,
    /// channel -> raw 14-bit pitch bend events.
    pub pitch_bends: BTreeMap<u8, Vec<PitchBendEvent>>,
    /// channel -> RPN 0,0 (Pitch Bend Sensitivity) changes.
    pub pitch_bend_ranges: BTreeMap<u8, Vec<PitchBendRangeEvent>>,
    /// Number of RPN 0,0 Data Entry events seen.
    pub rpn_pitch_bend_range_count: usize,
    /// Number of paired notes before channel/filter/transpose processing.
    pub note_count: usize,
    pub notes_per_channel: BTreeMap<u8, usize>,
    pub programs: BTreeMap<u8, BTreeSet<u8>>,
    pub initial_bpm: Option<f64>,
    /// Effective tempo map, sorted by tick. A tick-0 entry is synthesized at
    /// 120 BPM when the file's first tempo event is later than tick 0.
    pub tempo_map: Vec<TempoEventInfo>,
    pub tempo_event_count: usize,
    pub time_signatures: Vec<TimeSignatureInfo>,
    pub sustain_event_count: usize,
    pub pitch_bend_event_count: usize,
    pub min_pitch: u8,
    pub max_pitch: u8,
    pub min_duration_beats: f64,
    pub avg_duration_beats: f64,
    pub max_duration_beats: f64,
    /// End of the last note, in quarter-note beats.
    pub duration_beats: f64,
    pub max_polyphony: usize,
    pub zero_duration_notes: usize,
    pub overlapping_same_pitch_pairs: usize,
    pub unmatched_note_offs: usize,
    pub unterminated_note_ons: usize,
    pub warnings: Vec<String>,
}

impl Default for MidiStats {
    fn default() -> Self {
        Self {
            format: String::new(),
            ppq: 0,
            track_count: 0,
            track_names: Vec::new(),
            track_names_by_index: BTreeMap::new(),
            track_channels: BTreeMap::new(),
            controllers: BTreeMap::new(),
            pitch_bends: BTreeMap::new(),
            pitch_bend_ranges: BTreeMap::new(),
            rpn_pitch_bend_range_count: 0,
            note_count: 0,
            notes_per_channel: BTreeMap::new(),
            programs: BTreeMap::new(),
            initial_bpm: None,
            tempo_map: Vec::new(),
            tempo_event_count: 0,
            time_signatures: Vec::new(),
            sustain_event_count: 0,
            pitch_bend_event_count: 0,
            min_pitch: 0,
            max_pitch: 0,
            min_duration_beats: 0.0,
            avg_duration_beats: 0.0,
            max_duration_beats: 0.0,
            duration_beats: 0.0,
            max_polyphony: 0,
            zero_duration_notes: 0,
            overlapping_same_pitch_pairs: 0,
            unmatched_note_offs: 0,
            unterminated_note_ons: 0,
            warnings: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParsedMidi {
    pub notes: Vec<Note>,
    pub stats: MidiStats,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum RawNoteKind {
    On { key: u8, velocity: u8 },
    Off { key: u8, velocity: u8 },
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RawNoteEvent {
    pub(crate) tick: u64,
    pub(crate) order: u64,
    pub(crate) track: usize,
    pub(crate) channel: u8,
    pub(crate) kind: RawNoteKind,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PendingNote {
    pub(crate) start_tick: u64,
    pub(crate) velocity: u8,
}

impl fmt::Display for TimeSignatureInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{} @ tick {}",
            self.numerator, self.denominator, self.tick
        )
    }
}
