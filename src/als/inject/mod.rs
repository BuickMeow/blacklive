use super::*;

mod clone;
use clone::{clone_routed_group, clone_single_group};

pub(crate) enum PrototypeMode {
    Single { child: Element },
    Routed { instrument: Element, midi: Element },
}

pub(crate) fn resolve_prototype(
    tracks: &Element,
    prototype: &RawClip,
) -> Result<(Element, PrototypeMode, Element)> {
    let group_id = prototype
        .group_id
        .context("通道原型 Clip 不在 Group 内；需要把原型 Clip 放进 Group")?;
    let group_element_index = prototype
        .group_element_index
        .context("找不到通道原型 Clip 所在的 Group")?;
    let group = tracks
        .children
        .get(group_element_index)
        .and_then(XMLNode::as_element)
        .context("无法定位原型 Group")?;
    if track_kind(group) != TrackKind::Group {
        bail!("通道原型 Clip 的 TrackGroupId 指向的不是 GroupTrack");
    }

    let child_indices: Vec<usize> = tracks
        .children
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node.as_element().is_some_and(|track| {
                track_kind(track) == TrackKind::Midi && track_group_id(track) == Some(group_id)
            })
        })
        .map(|(index, _)| index)
        .collect();

    match child_indices.as_slice() {
        [child_index] => {
            if *child_index != prototype.track_element_index {
                bail!("原型 Clip 不在 Group 里的唯一 MIDI 轨上");
            }
            let child = tracks
                .children
                .get(*child_index)
                .and_then(XMLNode::as_element)
                .context("无法定位原型 MIDI 轨道")?;
            let clip = navigate(child, &prototype.clip_path)
                .context("无法定位原型 Clip 的 XML")?
                .clone();
            Ok((
                group.clone(),
                PrototypeMode::Single {
                    child: child.clone(),
                },
                clip,
            ))
        }
        [first, second] => {
            let mut instrument = None;
            let mut midi = None;
            for index in [*first, *second] {
                let track = tracks
                    .children
                    .get(index)
                    .and_then(XMLNode::as_element)
                    .context("无法定位原型子轨")?;
                if track_devices(track).1 {
                    instrument = Some((index, track));
                } else {
                    midi = Some((index, track));
                }
            }
            let (instrument_index, instrument) =
                instrument.context("路由模式下 Group 里需要一条乐器轨")?;
            let (midi_index, midi) = midi.context("路由模式下 Group 里需要一条 MIDI 轨")?;
            if midi_index != prototype.track_element_index {
                bail!("原型 Clip 应该放在路由模式的 MIDI 轨上，而不是乐器轨上");
            }
            let instrument_id = track_id(instrument).context("乐器轨缺少 Id")?;
            let routed_to = midi_routed_track_id(midi);
            if routed_to != Some(instrument_id) {
                bail!(
                    "路由模式的 MIDI 轨没有路由到 Group 内的乐器轨（当前 MIDI To: {:?}）",
                    routed_to
                );
            }
            let clip = navigate(midi, &prototype.clip_path)
                .context("无法定位路由模式 MIDI 轨的 Clip")?
                .clone();
            let _ = instrument_index;
            Ok((
                group.clone(),
                PrototypeMode::Routed {
                    instrument: instrument.clone(),
                    midi: midi.clone(),
                },
                clip,
            ))
        }
        _ => bail!(
            "通道原型 Group 需要 1 条乐器+Clip 轨，或 1 条乐器轨 + 1 条 MIDI 轨（MIDI To 指向乐器轨）；当前有 {} 条 MIDI 轨。",
            child_indices.len()
        ),
    }
}

pub(crate) fn midi_routed_track_id(track: &Element) -> Option<u32> {
    let target = track
        .get_child("DeviceChain")?
        .get_child("MidiOutputRouting")?
        .get_child("Target")
        .and_then(|node| attr(node, "Value"))?;
    let rest = target.strip_prefix("MidiOut/Track.")?;
    rest.split('/').next()?.parse().ok()
}

pub(crate) fn set_midi_output_to_track(
    track: &mut Element,
    instrument_id: u32,
    instrument_name: &str,
) {
    if let Some(routing) = find_descendant_mut(track, "MidiOutputRouting") {
        set_child_value(
            routing,
            "Target",
            &format!("MidiOut/Track.{instrument_id}/TrackIn"),
        );
        set_child_value(routing, "UpperDisplayString", instrument_name);
        set_child_value(routing, "LowerDisplayString", "Track In");
    }
}

pub(crate) fn inject_groups(
    root: &mut Element,
    req: GroupInjectRequest<'_>,
) -> Result<InjectionInfo> {
    validate_injection_request(&req)?;

    let mut warnings = Vec::new();
    let live_set = live_set_mut(root).context("模板里找不到 <LiveSet> 节点")?;
    let tempo_automation_written = set_tempo(live_set, req.bpm, req.tempo_map, &mut warnings);
    let mut pointee_allocator = PointeeAllocator::new(live_set);
    let mut next_track_id = {
        let tracks = live_set
            .get_child("Tracks")
            .context("模板里找不到 <LiveSet>/<Tracks> 节点")?;
        max_track_id(tracks) + 1
    };

    let mut new_tracks: Vec<XMLNode> = Vec::new();
    let mut remove_group_ids: HashSet<u32> = HashSet::new();
    let mut insert_at = usize::MAX;

    for group in req.groups {
        if group.children.is_empty() {
            continue;
        }

        let (prototype_group, mode, clip_template) = {
            let tracks = live_set
                .get_child("Tracks")
                .context("模板里找不到 <LiveSet>/<Tracks> 节点")?;
            resolve_prototype(tracks, &group.prototype)?
        };
        let group_id = group
            .prototype
            .group_id
            .context("通道原型 Clip 不在 Group 内")?;
        let group_element_index = group
            .prototype
            .group_element_index
            .context("找不到通道原型 Clip 所在的 Group")?;
        insert_at = insert_at.min(group_element_index);
        remove_group_ids.insert(group_id);

        let write_note_ids = clip_template
            .get_child("Notes")
            .and_then(|notes| notes.get_child("NoteIdGenerator"))
            .is_some();
        let instrument_track = match &mode {
            PrototypeMode::Single { child } => child,
            PrototypeMode::Routed { instrument, .. } => instrument,
        };
        warn_instrument(
            &group.group_name,
            instrument_track,
            req.strip_instrument_effects,
            &mut warnings,
        );

        let new_group_id = next_track_id;
        next_track_id += 1;
        new_tracks.push(XMLNode::Element(clone_group_track(
            prototype_group,
            &group.group_name,
            new_group_id,
            &mut pointee_allocator,
        )));

        match mode {
            PrototypeMode::Single {
                child: prototype_child,
            } => clone_single_group(
                group,
                prototype_child,
                &clip_template,
                write_note_ids,
                &req,
                new_group_id,
                &mut next_track_id,
                &mut pointee_allocator,
                &mut warnings,
                &mut new_tracks,
            )?,
            PrototypeMode::Routed { instrument, midi } => clone_routed_group(
                group,
                instrument,
                midi,
                &clip_template,
                write_note_ids,
                &req,
                new_group_id,
                &mut next_track_id,
                &mut pointee_allocator,
                &mut warnings,
                &mut new_tracks,
            )?,
        }
    }

    if insert_at == usize::MAX {
        bail!("没有可写入的通道原型 Group");
    }

    install_tracks(live_set, new_tracks, &remove_group_ids, insert_at)?;
    set_child_value(
        live_set,
        "NextPointeeId",
        &pointee_allocator.next.to_string(),
    );

    Ok(InjectionInfo {
        tempo_automation_written,
        warnings,
    })
}

fn validate_injection_request(req: &GroupInjectRequest<'_>) -> Result<()> {
    if req.groups.is_empty() {
        bail!("没有可写入的通道分组");
    }
    if !req.clip_start_beats.is_finite() || req.clip_start_beats < 0.0 {
        bail!("内部错误：Clip 起点必须是非负数");
    }
    if !req.bpm.is_finite() || req.bpm <= 0.0 {
        bail!("内部错误：BPM 必须是正数");
    }
    for group in req.groups {
        for child in &group.children {
            if !child.clip_length_beats.is_finite()
                || (child.clip_length_beats <= 0.0 && !child.notes.is_empty())
            {
                bail!("内部错误：Clip 长度必须是正数");
            }
        }
    }
    Ok(())
}

fn warn_instrument(
    group_name: &str,
    instrument_track: &Element,
    strip_instrument_effects: bool,
    warnings: &mut Vec<String>,
) {
    let (devices, has_instrument) = track_devices(instrument_track);
    let prototype_track_name = track_name(instrument_track);
    if !has_instrument {
        warnings.push(format!(
            "通道 {} 的原型乐器轨「{}」没有检测到乐器设备（设备：{}）。",
            group_name,
            prototype_track_name,
            if devices.is_empty() {
                "无".to_string()
            } else {
                devices.join(", ")
            }
        ));
    } else if !looks_like_piano(&prototype_track_name, &devices) {
        warnings.push(format!(
            "通道 {} 的原型乐器轨「{}」没有检测到钢琴类音色（设备：{}）；输出会使用模板里的音色。",
            group_name,
            prototype_track_name,
            devices.join(", ")
        ));
    }
    let effects = track_effect_names(instrument_track);
    if effects.is_empty() {
        return;
    }
    if strip_instrument_effects {
        warnings.push(format!(
            "通道 {} 的原型乐器轨「{}」内的 Reverb/Delay 已关闭（{}）；Group/Return 效果保留。",
            group_name,
            prototype_track_name,
            effects.join(", ")
        ));
    } else {
        warnings.push(format!(
            "通道 {} 的原型乐器轨「{}」包含 Reverb/Delay（{}）；黑乐谱建议在模板里关掉并移到 Group 或共享 Return。",
            group_name,
            prototype_track_name,
            effects.join(", ")
        ));
    }
}

fn clone_group_track(
    mut group: Element,
    name: &str,
    new_group_id: u32,
    pointee_allocator: &mut PointeeAllocator,
) -> Element {
    remap_pointee_ids(&mut group, pointee_allocator);
    set_attr(&mut group, "Id", &new_group_id.to_string());
    set_track_name(&mut group, name);
    set_track_group_id(&mut group, -1);
    set_linked_track_group_id(&mut group, -1);
    set_track_unfolded(&mut group, true);
    group
}

fn install_tracks(
    live_set: &mut Element,
    mut new_tracks: Vec<XMLNode>,
    remove_group_ids: &HashSet<u32>,
    insert_at: usize,
) -> Result<()> {
    let tracks = child_mut(live_set, "Tracks").context("模板里找不到 <LiveSet>/<Tracks> 节点")?;
    let old_children = std::mem::take(&mut tracks.children);
    let mut new_children = Vec::with_capacity(old_children.len() + new_tracks.len());
    let mut inserted = false;
    for (index, node) in old_children.into_iter().enumerate() {
        if index == insert_at {
            new_children.append(&mut new_tracks);
            inserted = true;
        }
        let remove = node.as_element().is_some_and(|track| {
            track_id(track).is_some_and(|id| remove_group_ids.contains(&id))
                || track_group_id(track)
                    .is_some_and(|group_id| remove_group_ids.contains(&group_id))
        });
        if !remove {
            new_children.push(node);
        }
    }
    if !inserted {
        new_children.append(&mut new_tracks);
    }
    tracks.children = new_children;
    Ok(())
}

pub(crate) fn set_track_group_id(track: &mut Element, value: i64) {
    set_child_value(track, "TrackGroupId", &value.to_string());
}

pub(crate) fn set_linked_track_group_id(track: &mut Element, value: i64) {
    set_child_value(track, "LinkedTrackGroupId", &value.to_string());
}

pub(crate) fn set_track_unfolded(track: &mut Element, unfolded: bool) {
    set_child_value(
        track,
        "TrackUnfolded",
        if unfolded { "true" } else { "false" },
    );
}

pub(crate) fn set_group_output(track: &mut Element) {
    if let Some(routing) = find_descendant_mut(track, "AudioOutputRouting") {
        set_child_value(routing, "Target", "AudioOut/GroupTrack");
        set_child_value(routing, "UpperDisplayString", "Group");
    }
}

pub(crate) fn set_record_arm(track: &mut Element, armed: bool) {
    if let Some(recorder) = find_descendant_mut(track, "Recorder")
        && recorder.get_child("IsArmed").is_some()
    {
        set_child_value(recorder, "IsArmed", if armed { "true" } else { "false" });
    }
}

pub(crate) fn max_track_id(tracks: &Element) -> u32 {
    tracks
        .children
        .iter()
        .filter_map(XMLNode::as_element)
        .filter(|track| {
            matches!(
                track_kind(track),
                TrackKind::Midi | TrackKind::Group | TrackKind::Return
            )
        })
        .filter_map(track_id)
        .max()
        .unwrap_or(0)
}

pub(crate) fn strip_midi_clips(element: &mut Element) {
    element.children.retain(|node| {
        node.as_element()
            .is_none_or(|child| child.name != "MidiClip")
    });
    for child in element
        .children
        .iter_mut()
        .filter_map(XMLNode::as_mut_element)
    {
        strip_midi_clips(child);
    }
}

/// Remove the instrument from a cloned child track while keeping the track
/// itself. Used for source tracks that have MIDI events but no notes.
pub(crate) fn clear_track_devices(track: &mut Element) {
    if let Some(devices) = track
        .get_mut_child("DeviceChain")
        .and_then(|chain| chain.get_mut_child("DeviceChain"))
        .and_then(|chain| chain.get_mut_child("Devices"))
    {
        devices.children.clear();
    }
}
