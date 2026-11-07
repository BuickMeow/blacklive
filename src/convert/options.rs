use std::collections::BTreeSet;

pub struct ConvertOptions {
    /// Select a template clip by its index in `TemplateReport::clips`.
    /// Takes precedence over `template_clip_name`.
    pub template_clip_index: Option<usize>,
    /// Select a template clip by its name (case-insensitive exact, then substring).
    pub template_clip_name: Option<String>,
    /// `None` means all channels. The GM drum channel (0-based index 9, often
    /// called channel 10 in DAWs) is excluded unless `include_drum_channel`.
    pub channels: Option<BTreeSet<u8>>,
    pub include_drum_channel: bool,
    pub transpose: i32,
    /// `None` uses the MIDI's initial tempo, then the template tempo.
    pub bpm_override: Option<f64>,
    /// Clamp every note to at least this many beats. 0 preserves the MIDI duration.
    pub min_note_beats: f64,
    /// Extra silence appended to the last note, in beats.
    pub tail_beats: f64,
    /// Arrangement start position, in beats (0 = bar 1).
    pub start_beats: f64,
    /// Base name for generated clips; defaults to the MIDI file stem.
    pub clip_name: Option<String>,
    /// Cut a note short when the next note of the same pitch starts before it ends.
    pub trim_same_pitch: bool,
    /// Create one child track per source MIDI track inside each channel group.
    pub split_source_tracks: bool,
    /// Keep source tracks that have MIDI events but no notes as empty child tracks.
    pub keep_empty_tracks: bool,
    /// Write MIDI Pitch Bend as clip envelope automation.
    pub write_pitch_bend: bool,
    /// Fixed pitch bend range in semitones. `None` uses the maximum RPN range
    /// found in the MIDI (at least 2 semitones, capped at 48).
    pub pitch_bend_range: Option<f64>,
    /// Write MIDI CC automation. Temporarily off; CC-to-effect mapping is future work.
    pub write_cc_automation: bool,
    /// Bypass Reverb/Delay/Chorus/Echo devices inside the cloned instrument.
    pub strip_instrument_effects: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            template_clip_index: None,
            template_clip_name: None,
            channels: None,
            include_drum_channel: false,
            transpose: 0,
            bpm_override: None,
            min_note_beats: 0.0,
            tail_beats: 0.0,
            start_beats: 0.0,
            clip_name: None,
            trim_same_pitch: false,
            split_source_tracks: true,
            keep_empty_tracks: true,
            write_pitch_bend: true,
            pitch_bend_range: None,
            write_cc_automation: false,
            strip_instrument_effects: true,
        }
    }
}
