use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use crate::midi::{ControllerEvent, Note, PitchBendEvent, PitchBendRangeEvent};

pub(crate) use anyhow::{Context, Result, bail};
pub(crate) use std::collections::{HashMap, HashSet};
pub(crate) use xmltree::{Element, XMLNode};

const INSTRUMENT_TAGS: &[&str] = &[
    "InstrumentGroupDevice",
    "OriginalSimpler",
    "Simpler",
    "Sampler",
    "DrumGroupDevice",
    "DrumRack",
    "Meld",
    "Drift",
    "Operator",
    "Wavetable",
    "Bass",
    "Collision",
    "Electric",
    "Impulse",
    "Tension",
    "Analog",
    "ExternalInstrument",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipLocation {
    Arrangement,
    Session,
}

impl fmt::Display for ClipLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arrangement => f.write_str("Arrangement"),
            Self::Session => f.write_str("Session"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackKind {
    Midi,
    Group,
    Return,
    Other,
}

impl fmt::Display for TrackKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Midi => f.write_str("MIDI"),
            Self::Group => f.write_str("Group"),
            Self::Return => f.write_str("Return"),
            Self::Other => f.write_str("Other"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClipInfo {
    /// Index in `TemplateReport::clips`, stable for the same template.
    pub candidate_index: usize,
    pub track_index: usize,
    pub track_name: String,
    pub group_id: Option<u32>,
    pub group_name: Option<String>,
    pub location: ClipLocation,
    pub name: String,
    pub time: f64,
    pub current_start: f64,
    pub current_end: f64,
    pub loop_start: f64,
    pub loop_end: f64,
    pub loop_on: bool,
    pub note_count: usize,
    pub key_track_count: usize,
}

impl ClipInfo {
    pub fn length_beats(&self) -> f64 {
        let current = self.current_end - self.current_start;
        if current > 0.0 {
            current
        } else {
            (self.loop_end - self.loop_start).max(0.0)
        }
    }

    pub fn display_label(&self) -> String {
        let group = self
            .group_name
            .as_ref()
            .map(|name| format!("Group「{name}」/ "))
            .unwrap_or_default();
        format!(
            "{}{} / 轨道「{}」/ Clip「{}」（{} 音符, {} beats）",
            group,
            self.location,
            self.track_name,
            self.name,
            self.note_count,
            crate::format_num(self.length_beats())
        )
    }
}

#[derive(Clone, Debug)]
pub struct TrackInfo {
    pub index: usize,
    pub id: Option<u32>,
    pub name: String,
    pub kind: TrackKind,
    pub group_id: Option<u32>,
    pub group_name: Option<String>,
    pub devices: Vec<String>,
    pub has_instrument: bool,
    pub midi_target: Option<u32>,
    pub routed_to_instrument: bool,
}

#[derive(Clone, Debug)]
pub struct TemplateReport {
    pub path: PathBuf,
    pub major_version: String,
    pub minor_version: String,
    pub creator: String,
    pub revision: String,
    pub bpm: Option<f64>,
    pub tracks: Vec<TrackInfo>,
    pub clips: Vec<ClipInfo>,
    /// First Arrangement clip if present, otherwise the first Session clip.
    pub recommended_clip: Option<usize>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct RawClip {
    pub info: ClipInfo,
    pub track_element_index: usize,
    /// Raw child indices from the track element to the `<MidiClip>`.
    pub clip_path: Vec<usize>,
    pub group_id: Option<u32>,
    pub group_element_index: Option<usize>,
}

pub(crate) struct ChildPlan {
    pub source_track: u16,
    pub track_name: String,
    pub clip_name: String,
    pub notes: Vec<Note>,
    pub clip_length_beats: f64,
}

pub(crate) struct GroupPlan {
    pub channel: u8,
    pub group_name: String,
    pub prototype: RawClip,
    pub children: Vec<ChildPlan>,
}

pub(crate) struct GroupInjectRequest<'a> {
    pub groups: &'a [GroupPlan],
    pub clip_start_beats: f64,
    pub bpm: f64,
    pub time_signature: Option<(u8, u16)>,
    /// (arrangement beat, BPM) pairs. `None` writes a constant tempo only.
    pub tempo_map: Option<&'a [(f64, f64)]>,
    /// channel -> controller number -> absolute-tick events.
    pub controllers: &'a BTreeMap<u8, BTreeMap<u8, Vec<ControllerEvent>>>,
    /// channel -> raw 14-bit pitch bend events.
    pub pitch_bends: &'a BTreeMap<u8, Vec<PitchBendEvent>>,
    /// channel -> RPN 0,0 pitch bend sensitivity changes.
    pub pitch_bend_ranges: &'a BTreeMap<u8, Vec<PitchBendRangeEvent>>,
    /// Fixed device pitch bend range in semitones used for PB envelope scaling.
    pub pitch_bend_range: f64,
    pub ppq: u16,
    pub write_pitch_bend: bool,
    pub write_cc_automation: bool,
    pub strip_instrument_effects: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct InjectionInfo {
    pub tempo_automation_written: bool,
    pub warnings: Vec<String>,
}
mod clip;
mod devices;
mod inject;
mod inspect;
mod io;
mod pointee;
mod tempo;
mod validate;
mod xml;

#[cfg(test)]
mod tests;

pub use inspect::{inspect_template, inspect_template_bytes};
pub use io::{dump_als_bytes_xml, dump_template_xml, read_als_bytes, read_als_xml};

pub(crate) use clip::*;
pub(crate) use devices::*;
pub(crate) use inject::{inject_groups, midi_routed_track_id};
pub(crate) use inspect::{enumerate_clips, select_clip};
pub(crate) use io::{parse_xml, serialize_xml, write_als_atomic};
pub(crate) use pointee::{PointeeAllocator, is_pointee_holder, remap_pointee_ids};
pub(crate) use tempo::{read_tempo, set_tempo, template_bpm};
pub(crate) use validate::validate_live_set;
pub(crate) use xml::*;
