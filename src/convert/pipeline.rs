use super::*;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::als::{self, GroupInjectRequest, TemplateReport};
use crate::midi::parse_midi_file;

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
    let max_rpn_pb_range = parsed
        .stats
        .pitch_bend_ranges
        .values()
        .flatten()
        .map(|event| event.semitones)
        .fold(0.0f64, f64::max);
    let mut fixed_pb_range = opts
        .pitch_bend_range
        .unwrap_or_else(|| max_rpn_pb_range.max(2.0));
    if !fixed_pb_range.is_finite() || fixed_pb_range <= 0.0 {
        bail!("Pitch Bend 范围必须是正数");
    }
    if fixed_pb_range > 48.0 {
        warnings.push(format!(
            "Pitch Bend 范围 {} 半音超过 Simpler 上限 48，已按 48 半音输出。",
            crate::format_num(fixed_pb_range)
        ));
        fixed_pb_range = 48.0;
    }
    if parsed.stats.pitch_bend_event_count > 0 {
        warnings.push(format!(
            "Pitch Bend 使用固定 {} 半音范围；动态 RPN 范围已折算进包络。",
            crate::format_num(fixed_pb_range)
        ));
    }
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

    let (groups, max_clip_length) =
        build_groups(&parsed, &filtered, opts, &clips, selected, &clip_name);

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
            pitch_bend_ranges: &parsed.stats.pitch_bend_ranges,
            pitch_bend_range: fixed_pb_range,
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
