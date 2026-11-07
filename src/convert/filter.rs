use super::*;
use crate::midi::{Note, ParsedMidi};

pub(crate) struct FilteredNotes {
    pub(crate) notes: Vec<Note>,
    pub(crate) dropped_by_channel: usize,
    pub(crate) dropped_by_transpose: usize,
    pub(crate) clamped_short_notes: usize,
    pub(crate) trimmed_overlaps: usize,
}

pub(crate) fn filter_notes(
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

pub(crate) fn trim_same_pitch_overlaps(notes: &mut [Note]) -> usize {
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
