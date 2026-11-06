use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::als::{self, ChildPlan, GroupInjectRequest, GroupPlan, TemplateReport};
use crate::midi::{MidiStats, Note, ParsedMidi, parse_midi_file};

#[derive(Clone, Debug)]
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
            write_cc_automation: false,
            strip_instrument_effects: true,
        }
    }
}

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

enum TemplateSource<'a> {
    File(&'a Path),
    Embedded,
}

impl TemplateSource<'_> {
    fn path(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path),
            Self::Embedded => None,
        }
    }

    fn load(&self) -> Result<(String, Vec<u8>)> {
        match self {
            Self::File(path) => Ok((path.display().to_string(), als::read_als_xml(path)?)),
            Self::Embedded => Ok((
                crate::DEFAULT_TEMPLATE_LABEL.to_string(),
                als::read_als_bytes(crate::DEFAULT_TEMPLATE_BYTES, crate::DEFAULT_TEMPLATE_LABEL)?,
            )),
        }
    }
}

/// Run the complete MIDI -> template injection -> `.als` pipeline.
pub fn convert_midi_to_als(
    midi_path: &Path,
    template_path: &Path,
    output_path: &Path,
    opts: &ConvertOptions,
) -> Result<ConversionResult> {
    convert_midi_with_template(
        midi_path,
        TemplateSource::File(template_path),
        output_path,
        opts,
    )
}

/// Convert with the built-in Ac Piano Upright template, so callers do not have
/// to ship or select a separate `.als` file.
pub fn convert_midi_to_als_with_default_template(
    midi_path: &Path,
    output_path: &Path,
    opts: &ConvertOptions,
) -> Result<ConversionResult> {
    convert_midi_with_template(midi_path, TemplateSource::Embedded, output_path, opts)
}

fn convert_midi_with_template(
    midi_path: &Path,
    template: TemplateSource<'_>,
    output_path: &Path,
    opts: &ConvertOptions,
) -> Result<ConversionResult> {
    ensure_distinct_paths(midi_path, template.path(), output_path)?;

    let parsed = parse_midi_file(midi_path)?;
    let (template_label, template_xml) = template.load()?;
    let mut root = als::parse_xml(&template_xml)?;
    let clips = als::enumerate_clips(&root)?;
    let selected = als::select_clip(
        &clips,
        opts.template_clip_index,
        opts.template_clip_name.as_deref(),
    )?;

    let clip_name = opts
        .clip_name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            midi_path
                .file_stem()
                .map(|s| s.to_string_lossy().trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "MIDI".to_string());

    let mut warnings = parsed.stats.warnings.clone();
    if !opts.write_cc_automation {
        let cc_events: usize = parsed
            .stats
            .controllers
            .values()
            .flat_map(|controllers| controllers.values())
            .map(Vec::len)
            .sum();
        if cc_events > 0 {
            warnings.push(format!(
                "MIDI 含 {cc_events} 个 CC 事件；当前未写入 CC 自动化（CC→效果映射待实现）。"
            ));
        }
    }
    if let Some(bpm) = opts.bpm_override {
        warnings.retain(|warning| !warning.contains("有效变速点"));
        if parsed.stats.tempo_map.len() > 1 {
            warnings.push(format!(
                "使用固定 BPM {} 覆盖；MIDI 的 {} 个变速点不会写入 Tempo 自动化。",
                crate::format_num(bpm),
                parsed.stats.tempo_map.len()
            ));
        }
    }
    let filtered = filter_notes(&parsed, opts, &mut warnings)?;

    let template_bpm = als::template_bpm(&root);
    let bpm = opts
        .bpm_override
        .or(parsed.stats.initial_bpm)
        .or(template_bpm)
        .unwrap_or(120.0);
    if !bpm.is_finite() || bpm <= 0.0 {
        bail!("BPM 必须是正数");
    }
    if opts.bpm_override.is_none() && parsed.stats.initial_bpm.is_none() {
        warnings.push(format!(
            "MIDI 没有 Tempo 事件，使用模板 BPM {}。",
            crate::format_num(template_bpm.unwrap_or(120.0))
        ));
    }

    if !opts.start_beats.is_finite() {
        bail!("Arrangement 起点必须是有限数");
    }
    let start_beats = if opts.start_beats < 0.0 {
        warnings.push("Arrangement 起点不能为负数，已改为 0。".into());
        0.0
    } else {
        opts.start_beats
    };

    let time_signature = parsed
        .stats
        .time_signatures
        .first()
        .map(|ts| (ts.numerator, ts.denominator));

    let tempo_map_beats: Vec<(f64, f64)> = if opts.bpm_override.is_none() {
        let mut points = Vec::with_capacity(parsed.stats.tempo_map.len());
        for event in &parsed.stats.tempo_map {
            let beat = start_beats + event.tick as f64 / parsed.stats.ppq as f64;
            if points
                .last()
                .is_some_and(|(_, previous): &(f64, f64)| *previous == event.bpm)
            {
                continue;
            }
            points.push((beat, event.bpm));
        }
        points
    } else {
        Vec::new()
    };
    let tempo_map = if tempo_map_beats.len() > 1 {
        Some(tempo_map_beats.as_slice())
    } else {
        None
    };

    let tail = opts.tail_beats.max(0.0);
    let mut grouped: BTreeMap<u8, BTreeMap<u16, Vec<&Note>>> = BTreeMap::new();
    for note in &filtered.notes {
        let source_track = if opts.split_source_tracks {
            note.source_track
        } else {
            0
        };
        grouped
            .entry(note.channel)
            .or_default()
            .entry(source_track)
            .or_default()
            .push(note);
    }

    let mut groups = Vec::new();
    let mut max_clip_length = 0.0f64;
    for (channel, tracks) in grouped {
        let group_name = format!("A{:02}", channel + 1);
        let child_count = tracks.len();
        let mut children = Vec::new();
        let mut used_names = std::collections::HashSet::new();

        for (child_index, (source_track, notes)) in tracks.into_iter().enumerate() {
            let source_name = parsed
                .stats
                .track_names_by_index
                .get(&source_track)
                .cloned()
                .filter(|name| !name.trim().is_empty());
            let mut track_name =
                source_name.unwrap_or_else(|| format!("{group_name}-{:02}", child_index + 1));
            if !used_names.insert(track_name.clone()) {
                track_name = format!("{track_name}-{:02}", child_index + 1);
            }
            let clip_name = if child_count == 1 {
                clip_name.clone()
            } else {
                format!("{clip_name} {group_name}-{:02}", child_index + 1)
            };
            let length = notes
                .iter()
                .map(|note| note.start + note.duration)
                .fold(0.0, f64::max)
                + tail;
            max_clip_length = max_clip_length.max(length);
            children.push(ChildPlan {
                source_track,
                track_name,
                clip_name,
                notes: notes.into_iter().cloned().collect(),
                clip_length_beats: length,
            });
        }

        let prototype = clips
            .iter()
            .find(|clip| clip.info.group_name.as_deref() == Some(group_name.as_str()))
            .cloned()
            .unwrap_or_else(|| selected.clone());
        groups.push(GroupPlan {
            channel,
            group_name,
            prototype,
            children,
        });
    }

    if opts.keep_empty_tracks && opts.split_source_tracks {
        let mut group_indices: BTreeMap<u8, usize> = BTreeMap::new();
        for (index, group) in groups.iter().enumerate() {
            group_indices.insert(group.channel, index);
        }
        for (source_track, channels) in &parsed.stats.track_channels {
            for channel in channels {
                let Some(&group_index) = group_indices.get(channel) else {
                    continue;
                };
                let group = &mut groups[group_index];
                if group
                    .children
                    .iter()
                    .any(|child| child.source_track == *source_track)
                {
                    continue;
                }
                let source_name = parsed
                    .stats
                    .track_names_by_index
                    .get(source_track)
                    .cloned()
                    .filter(|name| !name.trim().is_empty());
                let mut track_name = source_name
                    .unwrap_or_else(|| format!("{}-E{:02}", group.group_name, source_track + 1));
                if group
                    .children
                    .iter()
                    .any(|child| child.track_name == track_name)
                {
                    track_name = format!("{track_name} [src {}]", source_track + 1);
                }
                group.children.push(ChildPlan {
                    source_track: *source_track,
                    track_name: track_name.clone(),
                    clip_name: track_name,
                    notes: Vec::new(),
                    clip_length_beats: 0.0,
                });
            }
        }
        for group in &mut groups {
            group.children.sort_by_key(|child| child.source_track);
        }
    }

    if groups.is_empty() {
        bail!("没有可写入的通道分组");
    }

    let group_results: Vec<GroupResult> = groups
        .iter()
        .map(|group| GroupResult {
            channel: group.channel,
            name: group.group_name.clone(),
            tracks: group
                .children
                .iter()
                .map(|child| TrackResult {
                    name: child.track_name.clone(),
                    source_track: child.source_track,
                    note_count: child.notes.len(),
                    clip_length_beats: child.clip_length_beats,
                })
                .collect(),
        })
        .collect();

    let injection = als::inject_groups(
        &mut root,
        GroupInjectRequest {
            groups: &groups,
            clip_start_beats: start_beats,
            bpm,
            time_signature,
            tempo_map,
            controllers: &parsed.stats.controllers,
            pitch_bends: &parsed.stats.pitch_bends,
            ppq: parsed.stats.ppq,
            write_pitch_bend: opts.write_pitch_bend,
            write_cc_automation: opts.write_cc_automation,
            strip_instrument_effects: opts.strip_instrument_effects,
        },
    )?;
    warnings.extend(injection.warnings);
    let tempo_automation = injection.tempo_automation_written;

    als::validate_live_set(&root)?;

    let output_xml = als::serialize_xml(&root)?;
    // Re-parse and validate before writing, so an XML serialization or ID bug
    // can never produce a broken .als silently.
    let reparsed = als::parse_xml(&output_xml).context("输出 XML 自检失败")?;
    als::validate_live_set(&reparsed).context("输出 ID 自检失败")?;
    als::write_als_atomic(output_path, &output_xml)?;

    Ok(ConversionResult {
        output_path: output_path.to_path_buf(),
        template_label,
        midi_path: midi_path.to_path_buf(),
        template_clip: selected.info.display_label(),
        bpm,
        tempo_automation,
        clip_start_beats: start_beats,
        clip_length_beats: max_clip_length,
        source_note_count: parsed.stats.note_count,
        used_note_count: filtered.notes.len(),
        dropped_by_channel: filtered.dropped_by_channel,
        dropped_by_transpose: filtered.dropped_by_transpose,
        clamped_short_notes: filtered.clamped_short_notes,
        trimmed_overlaps: filtered.trimmed_overlaps,
        groups: group_results,
        midi_stats: parsed.stats,
        warnings,
    })
}

struct FilteredNotes {
    notes: Vec<Note>,
    dropped_by_channel: usize,
    dropped_by_transpose: usize,
    clamped_short_notes: usize,
    trimmed_overlaps: usize,
}

fn filter_notes(
    parsed: &ParsedMidi,
    opts: &ConvertOptions,
    warnings: &mut Vec<String>,
) -> Result<FilteredNotes> {
    if !opts.min_note_beats.is_finite() || opts.min_note_beats < 0.0 {
        bail!("最小音符时值必须是非负的有限数");
    }
    if !opts.tail_beats.is_finite() || opts.tail_beats < 0.0 {
        bail!("尾部留白必须是非负的有限数");
    }

    let mut notes = Vec::with_capacity(parsed.notes.len());
    let mut dropped_by_channel = 0usize;
    let mut dropped_drum_notes = 0usize;
    let mut clamped_short_notes = 0usize;

    for note in &parsed.notes {
        let keep = match &opts.channels {
            Some(channels) => channels.contains(&note.channel),
            None => !(note.channel == 9 && !opts.include_drum_channel),
        };
        if !keep {
            dropped_by_channel += 1;
            if note.channel == 9 && opts.channels.is_none() && !opts.include_drum_channel {
                dropped_drum_notes += 1;
            }
            continue;
        }

        let key = note.key as i32 + opts.transpose;
        if !(0..=127).contains(&key) {
            continue;
        }

        let mut out = note.clone();
        out.key = key as u8;
        out.duration = out.duration.max(1e-6);
        if opts.min_note_beats > 0.0 && out.duration < opts.min_note_beats {
            out.duration = opts.min_note_beats;
            clamped_short_notes += 1;
        }
        notes.push(out);
    }

    let dropped_by_transpose = parsed
        .notes
        .len()
        .saturating_sub(notes.len())
        .saturating_sub(dropped_by_channel);

    if dropped_drum_notes > 0 {
        warnings.push(format!(
            "默认排除了 GM 鼓通道（0-based index 9）的 {dropped_drum_notes} 个音符。若模板是鼓组，请勾选「包含鼓通道」。"
        ));
    }
    if opts.channels.is_some() && dropped_by_channel > 0 {
        warnings.push(format!(
            "通道过滤排除了 {dropped_by_channel} 个音符，保留 {} 个。",
            notes.len()
        ));
    }
    if dropped_by_transpose > 0 {
        warnings.push(format!(
            "移调 {:+} 后半音后有 {dropped_by_transpose} 个音符超出 MIDI 0–127，已丢弃。",
            opts.transpose
        ));
    }
    if opts.min_note_beats > 0.0 {
        warnings.push(format!(
            "最小音符时值设为 {} beats；短于该值的音符已被拉长。",
            crate::format_num(opts.min_note_beats)
        ));
    }

    let trimmed_overlaps = if opts.trim_same_pitch {
        let count = trim_same_pitch_overlaps(&mut notes);
        if count > 0 {
            warnings.push(format!(
                "同音高重叠：截短了 {count} 个前音，避免同 key 叠音。"
            ));
        }
        count
    } else {
        0
    };

    if notes.is_empty() {
        let channels = parsed
            .stats
            .notes_per_channel
            .keys()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "过滤后没有音符可写入。原始 MIDI 通道：{}；请检查通道过滤/移调设置。",
            if channels.is_empty() {
                "无".to_string()
            } else {
                channels
            }
        );
    }

    notes.sort_by(|a, b| {
        a.start
            .total_cmp(&b.start)
            .then_with(|| a.key.cmp(&b.key))
            .then_with(|| b.duration.total_cmp(&a.duration))
            .then_with(|| b.velocity.total_cmp(&a.velocity))
    });

    Ok(FilteredNotes {
        notes,
        dropped_by_channel,
        dropped_by_transpose,
        clamped_short_notes,
        trimmed_overlaps,
    })
}

fn trim_same_pitch_overlaps(notes: &mut [Note]) -> usize {
    notes.sort_by(|a, b| {
        a.key
            .cmp(&b.key)
            .then_with(|| a.start.total_cmp(&b.start))
            .then_with(|| b.duration.total_cmp(&a.duration))
    });

    let mut trimmed = 0usize;
    let mut group_start = 0usize;
    while group_start < notes.len() {
        let key = notes[group_start].key;
        let mut group_end = group_start + 1;
        while group_end < notes.len() && notes[group_end].key == key {
            group_end += 1;
        }

        for i in group_start..group_end.saturating_sub(1) {
            let next_start = notes[i + 1].start;
            let start = notes[i].start;
            if next_start > start && start + notes[i].duration > next_start {
                notes[i].duration = next_start - start;
                trimmed += 1;
            }
        }
        group_start = group_end;
    }
    trimmed
}

/// Parse "0,1,2", "0-3", or "" / "all" into a channel set.
pub fn parse_channel_filter(input: &str) -> Result<Option<BTreeSet<u8>>> {
    let input = input.trim();
    if input.is_empty() || input.eq_ignore_ascii_case("all") {
        return Ok(None);
    }

    let mut channels = BTreeSet::new();
    for part in input.split([',', '，']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((a, b)) = part.split_once('-') {
            let a = parse_channel(a)?;
            let b = parse_channel(b)?;
            if a > b {
                bail!("通道范围 {a}-{b} 顺序反了");
            }
            for ch in a..=b {
                channels.insert(ch);
            }
        } else {
            channels.insert(parse_channel(part)?);
        }
    }
    if channels.is_empty() {
        Ok(None)
    } else {
        Ok(Some(channels))
    }
}

fn parse_channel(s: &str) -> Result<u8> {
    let ch: u8 = s
        .trim()
        .parse()
        .with_context(|| format!("无效的 MIDI 通道「{s}」"))?;
    if ch > 15 {
        bail!("MIDI 通道必须在 0–15 之间，收到 {ch}");
    }
    Ok(ch)
}

fn ensure_distinct_paths(midi: &Path, template: Option<&Path>, output: &Path) -> Result<()> {
    if let Some(template) = template {
        if output == template {
            bail!("输出文件不能覆盖模板 .als");
        }
        if let (Ok(a), Ok(b)) = (
            std::fs::canonicalize(output),
            std::fs::canonicalize(template),
        ) && a == b
        {
            bail!("输出文件不能覆盖模板 .als");
        }
    }
    if output == midi {
        bail!("输出文件不能覆盖输入 MIDI");
    }
    if let (Ok(a), Ok(b)) = (std::fs::canonicalize(output), std::fs::canonicalize(midi))
        && a == b
    {
        bail!("输出文件不能覆盖输入 MIDI");
    }
    Ok(())
}

/// Convenience for the GUI: inspect a template and return both the report and
/// the raw candidate count. Errors are already in user-facing Chinese.
pub fn inspect_template(path: &Path) -> Result<TemplateReport> {
    als::inspect_template(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_filter_accepts_ranges() {
        let parsed = parse_channel_filter("0,2-4, 7").unwrap().unwrap();
        assert_eq!(parsed, BTreeSet::from([0, 2, 3, 4, 7]));
        assert!(parse_channel_filter("").unwrap().is_none());
        assert!(parse_channel_filter("all").unwrap().is_none());
        assert!(parse_channel_filter("16").is_err());
    }

    #[test]
    fn trim_overlaps_shortens_previous_note() {
        let mut notes = vec![
            Note {
                key: 60,
                start: 0.0,
                duration: 2.0,
                velocity: 100.0,
                off_velocity: 64.0,
                channel: 0,
                source_track: 0,
            },
            Note {
                key: 60,
                start: 1.0,
                duration: 1.0,
                velocity: 90.0,
                off_velocity: 64.0,
                channel: 0,
                source_track: 0,
            },
        ];
        assert_eq!(trim_same_pitch_overlaps(&mut notes), 1);
        assert_eq!(notes[0].duration, 1.0);
    }
}
