use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::path::Path;

use anyhow::{Context, Result, bail};
use midly::{Format, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

/// One note in Ableton's beat-based coordinate system.
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
enum RawNoteKind {
    On { key: u8, velocity: u8 },
    Off { key: u8, velocity: u8 },
}

#[derive(Clone, Copy, Debug)]
struct RawNoteEvent {
    tick: u64,
    order: u64,
    track: usize,
    channel: u8,
    kind: RawNoteKind,
}

#[derive(Clone, Copy, Debug)]
struct PendingNote {
    start_tick: u64,
    velocity: u8,
}

pub fn parse_midi_file(path: &Path) -> Result<ParsedMidi> {
    let bytes =
        std::fs::read(path).with_context(|| format!("读取 MIDI 文件失败: {}", path.display()))?;
    parse_midi_bytes(&bytes).with_context(|| format!("解析 MIDI 失败: {}", path.display()))
}

pub fn parse_midi_bytes(bytes: &[u8]) -> Result<ParsedMidi> {
    let smf = Smf::parse(bytes).context("不是有效的标准 MIDI 文件")?;

    let ppq = match smf.header.timing {
        Timing::Metrical(ppq) => ppq.as_int(),
        Timing::Timecode(fps, subframe) => {
            bail!(
                "不支持 SMPTE 时间码 MIDI（fps={:?}, subframe={}）；请先在 DAW 中转成 PPQ/Metrical 时间基准",
                fps,
                subframe
            );
        }
    };
    if ppq == 0 {
        bail!("MIDI 的 PPQ 为 0，无法换算 Beat");
    }

    let format = match smf.header.format {
        Format::SingleTrack => "0 (single track)".to_string(),
        Format::Parallel => "1 (parallel tracks)".to_string(),
        Format::Sequential => {
            bail!("不支持 MIDI Format 2（Sequential）；该格式的各轨不是同时播放的")
        }
    };

    let mut stats = MidiStats {
        format,
        ppq,
        track_count: smf.tracks.len(),
        ..Default::default()
    };

    let mut raw_events: Vec<RawNoteEvent> = Vec::new();
    let mut tempo_events: Vec<(u64, u64, f64)> = Vec::new();
    let mut time_signatures: Vec<TimeSignatureInfo> = Vec::new();
    let mut order = 0u64;

    for (track_idx, track) in smf.tracks.iter().enumerate() {
        let mut tick = 0u64;
        let mut track_name: Option<String> = None;

        for event in track {
            tick = tick.saturating_add(event.delta.as_int() as u64);
            order = order.wrapping_add(1);

            match event.kind {
                TrackEventKind::Midi { channel, message } => {
                    let channel = channel.as_int();
                    stats
                        .track_channels
                        .entry(track_idx as u16)
                        .or_default()
                        .insert(channel);
                    match message {
                        MidiMessage::NoteOn { key, vel } => {
                            let key = key.as_int();
                            let velocity = vel.as_int();
                            if velocity == 0 {
                                raw_events.push(RawNoteEvent {
                                    tick,
                                    order,
                                    track: track_idx,
                                    channel,
                                    kind: RawNoteKind::Off { key, velocity: 0 },
                                });
                            } else {
                                raw_events.push(RawNoteEvent {
                                    tick,
                                    order,
                                    track: track_idx,
                                    channel,
                                    kind: RawNoteKind::On { key, velocity },
                                });
                            }
                        }
                        MidiMessage::NoteOff { key, vel } => {
                            raw_events.push(RawNoteEvent {
                                tick,
                                order,
                                track: track_idx,
                                channel,
                                kind: RawNoteKind::Off {
                                    key: key.as_int(),
                                    velocity: vel.as_int(),
                                },
                            });
                        }
                        MidiMessage::Controller { controller, value } => {
                            let controller = controller.as_int();
                            let value = value.as_int();
                            if controller == 64 {
                                stats.sustain_event_count += 1;
                            }
                            stats
                                .controllers
                                .entry(channel)
                                .or_default()
                                .entry(controller)
                                .or_default()
                                .push(ControllerEvent { tick, value });
                        }
                        MidiMessage::PitchBend { bend } => {
                            stats.pitch_bend_event_count += 1;
                            stats
                                .pitch_bends
                                .entry(channel)
                                .or_default()
                                .push(PitchBendEvent {
                                    tick,
                                    value: bend.0.as_int(),
                                });
                        }
                        MidiMessage::ProgramChange { program } => {
                            stats
                                .programs
                                .entry(channel)
                                .or_default()
                                .insert(program.as_int());
                        }
                        _ => {}
                    }
                }
                TrackEventKind::Meta(meta) => match meta {
                    MetaMessage::Tempo(us_per_quarter) => {
                        let bpm = 60_000_000.0 / us_per_quarter.as_int() as f64;
                        stats.tempo_event_count += 1;
                        tempo_events.push((tick, order, bpm));
                    }
                    MetaMessage::TimeSignature(numerator, den_pow, _clocks, _32nds) => {
                        time_signatures.push(TimeSignatureInfo {
                            tick,
                            numerator,
                            denominator: 2u16.saturating_pow(den_pow as u32),
                        });
                    }
                    MetaMessage::TrackName(name) if track_name.is_none() => {
                        track_name = Some(String::from_utf8_lossy(name).trim().to_string());
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if let Some(name) = track_name.filter(|s| !s.is_empty()) {
            stats
                .track_names_by_index
                .insert(track_idx as u16, name.clone());
            stats.track_names.push(name);
        }
    }

    tempo_events.sort_by_key(|(tick, order, _)| (*tick, *order));
    let mut tempo_by_tick: BTreeMap<u64, f64> = BTreeMap::new();
    for (tick, _, bpm) in tempo_events {
        // Later events at the same tick win, matching MIDI playback semantics.
        tempo_by_tick.insert(tick, bpm);
    }
    if !tempo_by_tick.is_empty() && !tempo_by_tick.contains_key(&0) {
        // MIDI's default tempo before the first tempo event is 120 BPM.
        tempo_by_tick.insert(0, 120.0);
    }
    stats.initial_bpm = tempo_by_tick.get(&0).copied();
    stats.tempo_map = tempo_by_tick
        .into_iter()
        .map(|(tick, bpm)| TempoEventInfo { tick, bpm })
        .collect();
    if stats.tempo_map.len() > 1 {
        stats.warnings.push(format!(
            "检测到 {} 个有效变速点；转换时会写入 Ableton Tempo 自动化（不再忽略）。",
            stats.tempo_map.len()
        ));
    }

    time_signatures.sort_by_key(|t| t.tick);
    time_signatures.dedup_by(|a, b| {
        a.tick == b.tick && a.numerator == b.numerator && a.denominator == b.denominator
    });
    stats.time_signatures = time_signatures;
    if stats.time_signatures.len() > 1 {
        stats.warnings.push(format!(
            "检测到 {} 个拍号；当前只把第一个拍号写入 Clip 显示，播放不受拍号影响。",
            stats.time_signatures.len()
        ));
    }

    if stats.programs.values().any(|set| !set.is_empty()) {
        stats.warnings.push(
            "检测到 Program Change；当前按通道分组，Program Change 不会切换乐器。".to_string(),
        );
    }

    raw_events.sort_by_key(|event| (event.tick, event.order));

    // LIFO pairing (same as yinhe's black-MIDI parser): a Note Off releases the
    // most recently started un-released note of that (channel, key). MIDI itself
    // does not define which overlapping same-key Note On a Note Off belongs to;
    // LIFO matches how most synthesizers implement a per-key note stack.
    let mut pending: HashMap<(usize, u8, u8), Vec<PendingNote>> = HashMap::new();
    let mut notes: Vec<Note> = Vec::with_capacity(raw_events.len() / 2 + 1);

    for event in raw_events {
        match event.kind {
            RawNoteKind::On { key, velocity } => {
                let stack = pending
                    .entry((event.track, event.channel, key))
                    .or_default();
                if !stack.is_empty() {
                    stats.overlapping_same_pitch_pairs += 1;
                }
                stack.push(PendingNote {
                    start_tick: event.tick,
                    velocity,
                });
            }
            RawNoteKind::Off { key, velocity } => {
                let matched = pending
                    .get_mut(&(event.track, event.channel, key))
                    .and_then(|stack| stack.pop());
                if let Some(start) = matched {
                    let duration_ticks = event.tick.saturating_sub(start.start_tick);
                    if duration_ticks == 0 {
                        stats.zero_duration_notes += 1;
                    }
                    let effective_ticks = duration_ticks.max(1);
                    notes.push(Note {
                        key,
                        start: start.start_tick as f64 / ppq as f64,
                        duration: effective_ticks as f64 / ppq as f64,
                        velocity: start.velocity as f64,
                        off_velocity: if velocity == 0 { 64.0 } else { velocity as f64 },
                        channel: event.channel,
                        source_track: event.track as u16,
                    });
                } else {
                    stats.unmatched_note_offs += 1;
                }
            }
        }
    }

    // Files that omit final note-offs are common. Dropping these notes is much
    // safer than extending them to the end of the file: a Note On at beat 10 in
    // a 500-beat black MIDI would otherwise become a 490-beat stuck note. This
    // also matches yinhe's black-MIDI parser behavior.
    for stack in pending.values() {
        stats.unterminated_note_ons += stack.len();
    }

    notes.sort_by(|a, b| {
        a.start
            .total_cmp(&b.start)
            .then_with(|| a.key.cmp(&b.key))
            .then_with(|| b.duration.total_cmp(&a.duration))
            .then_with(|| b.velocity.total_cmp(&a.velocity))
            .then_with(|| a.channel.cmp(&b.channel))
    });

    stats.note_count = notes.len();
    if stats.note_count == 0 {
        bail!("MIDI 中没有可用的 Note On/Off 音符");
    }

    for note in &notes {
        *stats.notes_per_channel.entry(note.channel).or_default() += 1;
    }

    stats.min_pitch = notes.iter().map(|n| n.key).min().unwrap_or(0);
    stats.max_pitch = notes.iter().map(|n| n.key).max().unwrap_or(0);
    stats.min_duration_beats = notes
        .iter()
        .map(|n| n.duration)
        .fold(f64::INFINITY, f64::min);
    stats.max_duration_beats = notes.iter().map(|n| n.duration).fold(0.0, f64::max);
    stats.avg_duration_beats =
        notes.iter().map(|n| n.duration).sum::<f64>() / stats.note_count as f64;
    stats.duration_beats = notes
        .iter()
        .map(|n| n.start + n.duration)
        .fold(0.0, f64::max);
    stats.max_polyphony = max_polyphony_ticks(&notes, ppq);

    if stats.zero_duration_notes > 0 {
        stats.warnings.push(format!(
            "{} 个音符的 Note On/Off 在同一 tick，已按 1 tick 的最短时值写入。",
            stats.zero_duration_notes
        ));
    }
    if stats.unmatched_note_offs > 0 {
        stats.warnings.push(format!(
            "{} 个 Note Off 找不到对应的 Note On，已忽略。",
            stats.unmatched_note_offs
        ));
    }
    if stats.unterminated_note_ons > 0 {
        stats.warnings.push(format!(
            "{} 个 Note On 没有 Note Off，已忽略（避免生成超长挂音）。",
            stats.unterminated_note_ons
        ));
    }

    Ok(ParsedMidi { notes, stats })
}

fn max_polyphony_ticks(notes: &[Note], ppq: u16) -> usize {
    let mut edges: Vec<(u64, i32)> = Vec::with_capacity(notes.len() * 2);
    for note in notes {
        let start = (note.start * ppq as f64).round() as u64;
        let end = ((note.start + note.duration) * ppq as f64).round() as u64;
        edges.push((start, 1));
        edges.push((end, -1));
    }
    // At the same tick, note-offs come before note-ons, so back-to-back notes do not count as overlap.
    edges.sort_by_key(|(tick, delta)| (*tick, *delta));
    let mut current = 0i32;
    let mut max = 0i32;
    for (_, delta) in edges {
        current += delta;
        max = max.max(current);
    }
    max.max(0) as usize
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_polyphony_counts_off_before_on_at_same_tick() {
        let notes = vec![
            Note {
                key: 60,
                start: 0.0,
                duration: 1.0,
                velocity: 100.0,
                off_velocity: 64.0,
                channel: 0,
                source_track: 0,
            },
            Note {
                key: 60,
                start: 1.0,
                duration: 1.0,
                velocity: 100.0,
                off_velocity: 64.0,
                channel: 0,
                source_track: 0,
            },
        ];
        assert_eq!(max_polyphony_ticks(&notes, 480), 1);
    }
}
