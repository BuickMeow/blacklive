use super::*;

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
        let (devices, has_instrument) = track_devices(instrument_track);
        let prototype_track_name = track_name(instrument_track);
        if !has_instrument {
            warnings.push(format!(
                "通道 {} 的原型乐器轨「{}」没有检测到乐器设备（设备：{}）。",
                group.group_name,
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
                group.group_name,
                prototype_track_name,
                devices.join(", ")
            ));
        }
        let effects = track_effect_names(instrument_track);
        if !effects.is_empty() {
            if req.strip_instrument_effects {
                warnings.push(format!(
                    "通道 {} 的原型乐器轨「{}」内的 Reverb/Delay 已关闭（{}）；Group/Return 效果保留。",
                    group.group_name,
                    prototype_track_name,
                    effects.join(", ")
                ));
            } else {
                warnings.push(format!(
                    "通道 {} 的原型乐器轨「{}」包含 Reverb/Delay（{}）；黑乐谱建议在模板里关掉并移到 Group 或共享 Return。",
                    group.group_name,
                    prototype_track_name,
                    effects.join(", ")
                ));
            }
        }

        let new_group_id = next_track_id;
        next_track_id += 1;
        let mut new_group = prototype_group;
        remap_pointee_ids(&mut new_group, &mut pointee_allocator);
        set_attr(&mut new_group, "Id", &new_group_id.to_string());
        set_track_name(&mut new_group, &group.group_name);
        set_track_group_id(&mut new_group, -1);
        set_linked_track_group_id(&mut new_group, -1);
        set_track_unfolded(&mut new_group, true);
        new_tracks.push(XMLNode::Element(new_group));

        match mode {
            PrototypeMode::Single {
                child: prototype_child,
            } => {
                for child in &group.children {
                    let new_child_id = next_track_id;
                    next_track_id += 1;

                    let mut new_child = prototype_child.clone();
                    remap_pointee_ids(&mut new_child, &mut pointee_allocator);
                    set_attr(&mut new_child, "Id", &new_child_id.to_string());
                    set_track_name(&mut new_child, &child.track_name);
                    set_track_group_id(&mut new_child, new_group_id as i64);
                    set_linked_track_group_id(&mut new_child, -1);
                    set_track_unfolded(&mut new_child, true);
                    set_group_output(&mut new_child);
                    set_record_arm(&mut new_child, false);
                    if req.strip_instrument_effects {
                        bypass_instrument_effects(&mut new_child);
                    }
                    if req.write_pitch_bend {
                        let _ = set_pitch_bend_range(&mut new_child, req.pitch_bend_range);
                    }

                    let targets = controller_targets(&new_child);
                    strip_midi_clips(&mut new_child);
                    if child.notes.is_empty() {
                        clear_track_devices(&mut new_child);
                        new_tracks.push(XMLNode::Element(new_child));
                        continue;
                    }
                    let events = ensure_arranger_events(&mut new_child)?;
                    events
                        .children
                        .push(XMLNode::Element(clip_template.clone()));
                    let new_clip = events
                        .children
                        .last_mut()
                        .and_then(XMLNode::as_mut_element)
                        .context("写入 Arrangement Clip 失败")?;

                    edit_clip(
                        new_clip,
                        req.clip_start_beats,
                        child.clip_length_beats,
                        &child.clip_name,
                        req.time_signature,
                        &child.notes,
                        write_note_ids,
                    )?;
                    clear_clip_envelopes(new_clip);
                    if req.write_pitch_bend {
                        add_midi_automation(
                            new_clip,
                            group.channel,
                            req.controllers,
                            req.pitch_bends,
                            req.pitch_bend_ranges,
                            req.pitch_bend_range,
                            &targets,
                            req.ppq,
                            child.clip_length_beats,
                            req.write_cc_automation,
                            req.write_pitch_bend,
                            &mut warnings,
                        )?;
                    }

                    let notes_in_clip = count_descendants(new_clip, "MidiNoteEvent");
                    if notes_in_clip != child.notes.len() {
                        bail!(
                            "内部校验失败：通道 A{:02} 轨道「{}」写入 {} 个音符，但 Clip 中读到 {} 个",
                            group.channel + 1,
                            child.track_name,
                            child.notes.len(),
                            notes_in_clip
                        );
                    }

                    new_tracks.push(XMLNode::Element(new_child));
                }
            }
            PrototypeMode::Routed { instrument, midi } => {
                let instrument_name = format!("{} Inst", group.group_name);
                let new_instrument_id = next_track_id;
                next_track_id += 1;
                let mut new_instrument = instrument;
                remap_pointee_ids(&mut new_instrument, &mut pointee_allocator);
                set_attr(&mut new_instrument, "Id", &new_instrument_id.to_string());
                set_track_name(&mut new_instrument, &instrument_name);
                set_track_group_id(&mut new_instrument, new_group_id as i64);
                set_linked_track_group_id(&mut new_instrument, -1);
                set_track_unfolded(&mut new_instrument, true);
                set_group_output(&mut new_instrument);
                set_record_arm(&mut new_instrument, false);
                if req.strip_instrument_effects {
                    bypass_instrument_effects(&mut new_instrument);
                }
                if req.write_pitch_bend
                    && req
                        .pitch_bends
                        .get(&group.channel)
                        .is_some_and(|events| !events.is_empty())
                    && !set_pitch_bend_range(&mut new_instrument, req.pitch_bend_range)
                {
                    warnings.push(format!(
                        "通道 {} 的乐器没有 PitchBendRange 参数；PB 包络已按 {} 半音缩放，请在乐器里手动设置相同范围。",
                        group.group_name,
                        crate::format_num(req.pitch_bend_range)
                    ));
                }
                strip_midi_clips(&mut new_instrument);
                new_tracks.push(XMLNode::Element(new_instrument));

                // Put channel-wide automation on the longest clip so PB/CC
                // events near the end of the channel are not cut off when the
                // first source track is shorter. Empty tracks still stay empty.
                let automation_child = group
                    .children
                    .iter()
                    .enumerate()
                    .filter(|(_, child)| !child.notes.is_empty())
                    .max_by(|(_, a), (_, b)| a.clip_length_beats.total_cmp(&b.clip_length_beats))
                    .map(|(index, _)| index);

                for (index, child) in group.children.iter().enumerate() {
                    let new_midi_id = next_track_id;
                    next_track_id += 1;

                    let mut new_midi = midi.clone();
                    remap_pointee_ids(&mut new_midi, &mut pointee_allocator);
                    set_attr(&mut new_midi, "Id", &new_midi_id.to_string());
                    set_track_name(&mut new_midi, &child.track_name);
                    set_track_group_id(&mut new_midi, new_group_id as i64);
                    set_linked_track_group_id(&mut new_midi, -1);
                    set_track_unfolded(&mut new_midi, true);
                    set_record_arm(&mut new_midi, false);
                    set_midi_output_to_track(&mut new_midi, new_instrument_id, &instrument_name);

                    strip_midi_clips(&mut new_midi);
                    if child.notes.is_empty() {
                        new_tracks.push(XMLNode::Element(new_midi));
                        continue;
                    }
                    let targets = controller_targets(&new_midi);
                    let events = ensure_arranger_events(&mut new_midi)?;
                    events
                        .children
                        .push(XMLNode::Element(clip_template.clone()));
                    let new_clip = events
                        .children
                        .last_mut()
                        .and_then(XMLNode::as_mut_element)
                        .context("写入 Arrangement Clip 失败")?;

                    edit_clip(
                        new_clip,
                        req.clip_start_beats,
                        child.clip_length_beats,
                        &child.clip_name,
                        req.time_signature,
                        &child.notes,
                        write_note_ids,
                    )?;
                    clear_clip_envelopes(new_clip);
                    // All MIDI tracks feed the same instrument. Put channel-wide
                    // CC/Pitch Bend automation on one clip only.
                    if automation_child == Some(index)
                        && (req.write_cc_automation || req.write_pitch_bend)
                    {
                        add_midi_automation(
                            new_clip,
                            group.channel,
                            req.controllers,
                            req.pitch_bends,
                            req.pitch_bend_ranges,
                            req.pitch_bend_range,
                            &targets,
                            req.ppq,
                            child.clip_length_beats,
                            req.write_cc_automation,
                            req.write_pitch_bend,
                            &mut warnings,
                        )?;
                    }

                    let notes_in_clip = count_descendants(new_clip, "MidiNoteEvent");
                    if notes_in_clip != child.notes.len() {
                        bail!(
                            "内部校验失败：通道 A{:02} 轨道「{}」写入 {} 个音符，但 Clip 中读到 {} 个",
                            group.channel + 1,
                            child.track_name,
                            child.notes.len(),
                            notes_in_clip
                        );
                    }

                    new_tracks.push(XMLNode::Element(new_midi));
                }
            }
        }
    }

    if insert_at == usize::MAX {
        bail!("没有可写入的通道原型 Group");
    }

    {
        let tracks =
            child_mut(live_set, "Tracks").context("模板里找不到 <LiveSet>/<Tracks> 节点")?;
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
    }

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
