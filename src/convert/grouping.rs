use super::*;
use std::collections::{BTreeMap, HashSet};

use crate::als::{ChildPlan, GroupPlan, RawClip};
use crate::midi::{Note, ParsedMidi};

pub(crate) fn build_groups(
    parsed: &ParsedMidi,
    filtered: &FilteredNotes,
    opts: &ConvertOptions,
    clips: &[RawClip],
    selected: &RawClip,
    clip_name: &str,
) -> (Vec<GroupPlan>, f64) {
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
        let mut used_names = HashSet::new();

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
                clip_name.to_string()
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

    (groups, max_clip_length)
}
