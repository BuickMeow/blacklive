use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn clone_single_group(
    group: &GroupPlan,
    prototype_child: Element,
    clip_template: &Element,
    write_note_ids: bool,
    req: &GroupInjectRequest<'_>,
    new_group_id: u32,
    next_track_id: &mut u32,
    pointee_allocator: &mut PointeeAllocator,
    warnings: &mut Vec<String>,
    new_tracks: &mut Vec<XMLNode>,
) -> Result<()> {
    for child in &group.children {
        let new_child_id = *next_track_id;
        *next_track_id += 1;

        let mut new_child = prototype_child.clone();
        remap_pointee_ids(&mut new_child, pointee_allocator);
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

        if child.notes.is_empty() {
            strip_midi_clips(&mut new_child);
            clear_track_devices(&mut new_child);
            new_tracks.push(XMLNode::Element(new_child));
            continue;
        }

        write_child_clip(
            &mut new_child,
            child,
            clip_template,
            write_note_ids,
            req,
            group.channel,
            true,
            warnings,
        )?;
        new_tracks.push(XMLNode::Element(new_child));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn clone_routed_group(
    group: &GroupPlan,
    instrument: Element,
    midi: Element,
    clip_template: &Element,
    write_note_ids: bool,
    req: &GroupInjectRequest<'_>,
    new_group_id: u32,
    next_track_id: &mut u32,
    pointee_allocator: &mut PointeeAllocator,
    warnings: &mut Vec<String>,
    new_tracks: &mut Vec<XMLNode>,
) -> Result<()> {
    let instrument_name = format!("{} Inst", group.group_name);
    let new_instrument_id = *next_track_id;
    *next_track_id += 1;
    let mut new_instrument = instrument;
    remap_pointee_ids(&mut new_instrument, pointee_allocator);
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

    // Put channel-wide automation on the longest clip so PB/CC events near
    // the end of the channel are not cut off when the first source track is
    // shorter. Empty tracks still stay empty.
    let automation_child = group
        .children
        .iter()
        .enumerate()
        .filter(|(_, child)| !child.notes.is_empty())
        .max_by(|(_, a), (_, b)| a.clip_length_beats.total_cmp(&b.clip_length_beats))
        .map(|(index, _)| index);

    for (index, child) in group.children.iter().enumerate() {
        let new_midi_id = *next_track_id;
        *next_track_id += 1;

        let mut new_midi = midi.clone();
        remap_pointee_ids(&mut new_midi, pointee_allocator);
        set_attr(&mut new_midi, "Id", &new_midi_id.to_string());
        set_track_name(&mut new_midi, &child.track_name);
        set_track_group_id(&mut new_midi, new_group_id as i64);
        set_linked_track_group_id(&mut new_midi, -1);
        set_track_unfolded(&mut new_midi, true);
        set_record_arm(&mut new_midi, false);
        set_midi_output_to_track(&mut new_midi, new_instrument_id, &instrument_name);

        if child.notes.is_empty() {
            strip_midi_clips(&mut new_midi);
            new_tracks.push(XMLNode::Element(new_midi));
            continue;
        }

        write_child_clip(
            &mut new_midi,
            child,
            clip_template,
            write_note_ids,
            req,
            group.channel,
            automation_child == Some(index),
            warnings,
        )?;
        new_tracks.push(XMLNode::Element(new_midi));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_child_clip(
    track: &mut Element,
    child: &ChildPlan,
    clip_template: &Element,
    write_note_ids: bool,
    req: &GroupInjectRequest<'_>,
    channel: u8,
    write_automation: bool,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let targets = controller_targets(track);
    strip_midi_clips(track);
    let events = ensure_arranger_events(track)?;
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
    if write_automation && (req.write_cc_automation || req.write_pitch_bend) {
        add_midi_automation(
            new_clip,
            channel,
            req.controllers,
            req.pitch_bends,
            req.pitch_bend_ranges,
            req.pitch_bend_range,
            &targets,
            req.ppq,
            child.clip_length_beats,
            req.write_cc_automation,
            req.write_pitch_bend,
            warnings,
        )?;
    }

    let notes_in_clip = count_descendants(new_clip, "MidiNoteEvent");
    if notes_in_clip != child.notes.len() {
        bail!(
            "内部校验失败：通道 A{:02} 轨道「{}」写入 {} 个音符，但 Clip 中读到 {} 个",
            channel + 1,
            child.track_name,
            child.notes.len(),
            notes_in_clip
        );
    }
    Ok(())
}
