use super::*;

pub(crate) fn edit_clip(
    clip: &mut Element,
    start: f64,
    length: f64,
    clip_name: &str,
    time_signature: Option<(u8, u16)>,
    notes: &[Note],
    write_note_ids: bool,
) -> Result<()> {
    set_attr(clip, "Time", &crate::format_num(start));
    set_child_value(clip, "CurrentStart", &crate::format_num(start));
    set_child_value(clip, "CurrentEnd", &crate::format_num(start + length));

    let loop_el = ensure_child(clip, "Loop");
    set_child_value(loop_el, "LoopStart", "0");
    set_child_value(loop_el, "LoopEnd", &crate::format_num(length));
    set_child_value(loop_el, "StartRelative", "0");
    set_child_value(loop_el, "LoopOn", "false");
    set_child_value(loop_el, "OutMarker", &crate::format_num(length));
    set_child_value(loop_el, "HiddenLoopStart", "0");
    set_child_value(loop_el, "HiddenLoopEnd", &crate::format_num(length));

    set_child_value(clip, "Name", clip_name);
    if clip.get_child("Disabled").is_some() {
        set_child_value(clip, "Disabled", "false");
    }

    let scroller = ensure_child(clip, "ScrollerTimePreserver");
    set_child_value(scroller, "LeftTime", "0");
    set_child_value(scroller, "RightTime", &crate::format_num(length));

    if let Some(selection) = clip.get_mut_child("TimeSelection") {
        set_child_value(selection, "AnchorTime", "0");
        set_child_value(selection, "OtherTime", "0");
    }
    if let Some(freeze_start) = clip.get_mut_child("FreezeStart") {
        freeze_start.attributes.insert("Value".into(), "0".into());
    }
    if let Some(freeze_end) = clip.get_mut_child("FreezeEnd") {
        freeze_end.attributes.insert("Value".into(), "0".into());
    }

    if let Some((numerator, denominator)) = time_signature
        && let Some(sig) = clip
            .get_mut_child("TimeSignature")
            .and_then(|e| e.get_mut_child("TimeSignatures"))
            .and_then(|e| {
                e.children
                    .iter_mut()
                    .filter_map(XMLNode::as_mut_element)
                    .find(|e| e.name == "RemoteableTimeSignature")
            })
    {
        set_child_value(sig, "Numerator", &numerator.to_string());
        set_child_value(sig, "Denominator", &denominator.to_string());
    }

    replace_notes(clip, notes, write_note_ids)
}

pub(crate) fn replace_notes(
    clip: &mut Element,
    notes: &[Note],
    write_note_ids: bool,
) -> Result<()> {
    let notes_el = clip
        .get_mut_child("Notes")
        .context("模板 Clip 没有 <Notes> 节点，无法注入音符")?;

    let keytracks = build_keytracks(notes, write_note_ids);
    if let Some(existing) = notes_el.get_mut_child("KeyTracks") {
        *existing = keytracks;
    } else {
        notes_el.children.insert(0, XMLNode::Element(keytracks));
    }

    if let Some(store) = notes_el.get_mut_child("PerNoteEventStore")
        && let Some(lists) = store.get_mut_child("EventLists")
    {
        lists.children.clear();
    }
    if let Some(groups) = notes_el.get_mut_child("NoteProbabilityGroups") {
        groups.children.clear();
    }
    if let Some(generator) = notes_el
        .get_mut_child("ProbabilityGroupIdGenerator")
        .and_then(|g| g.get_mut_child("NextId"))
    {
        generator.attributes.insert("Value".into(), "1".into());
    }
    if let Some(generator) = notes_el
        .get_mut_child("NoteIdGenerator")
        .and_then(|g| g.get_mut_child("NextId"))
    {
        generator
            .attributes
            .insert("Value".into(), (notes.len() + 1).to_string());
    }

    Ok(())
}

pub(crate) fn build_keytracks(notes: &[Note], write_note_ids: bool) -> Element {
    use std::collections::BTreeMap;

    let mut by_key: BTreeMap<u8, Vec<&Note>> = BTreeMap::new();
    for note in notes {
        by_key.entry(note.key).or_default().push(note);
    }

    let mut keytracks = Element::new("KeyTracks");
    let mut note_id = 1usize;
    for (keytrack_id, (pitch, mut group)) in by_key.into_iter().enumerate() {
        group.sort_by(|a, b| {
            a.start
                .total_cmp(&b.start)
                .then_with(|| b.duration.total_cmp(&a.duration))
                .then_with(|| b.velocity.total_cmp(&a.velocity))
        });

        let mut keytrack = Element::new("KeyTrack");
        keytrack
            .attributes
            .insert("Id".into(), keytrack_id.to_string());

        let mut key_notes = Element::new("Notes");
        for note in group {
            let mut event = Element::new("MidiNoteEvent");
            event
                .attributes
                .insert("Time".into(), crate::format_num(note.start));
            event
                .attributes
                .insert("Duration".into(), crate::format_num(note.duration));
            event
                .attributes
                .insert("Velocity".into(), crate::format_num(note.velocity));
            event
                .attributes
                .insert("OffVelocity".into(), crate::format_num(note.off_velocity));
            if write_note_ids {
                event
                    .attributes
                    .insert("NoteId".into(), note_id.to_string());
                note_id += 1;
            }
            key_notes.children.push(XMLNode::Element(event));
        }

        let mut midi_key = Element::new("MidiKey");
        midi_key
            .attributes
            .insert("Value".into(), pitch.to_string());

        keytrack.children.push(XMLNode::Element(key_notes));
        keytrack.children.push(XMLNode::Element(midi_key));
        keytracks.children.push(XMLNode::Element(keytrack));
    }

    keytracks
}

pub(crate) fn clear_clip_envelopes(clip: &mut Element) {
    if let Some(envelopes) = clip.get_mut_child("Envelopes")
        && let Some(inner) = envelopes.get_mut_child("Envelopes")
    {
        inner.children.clear();
    }
}

pub(crate) fn controller_targets(track: &Element) -> BTreeMap<u8, String> {
    let Some(midi_controllers) = find_descendant(track, "MidiControllers") else {
        return BTreeMap::new();
    };
    let mut targets = BTreeMap::new();
    for node in &midi_controllers.children {
        let Some(target) = node.as_element() else {
            continue;
        };
        let Some(cc) = target
            .name
            .strip_prefix("ControllerTargets.")
            .and_then(|cc| cc.parse::<u8>().ok())
        else {
            continue;
        };
        let Some(id) = attr(target, "Id").filter(|id| !id.is_empty()) else {
            continue;
        };
        targets.insert(cc, id.to_string());
    }
    targets
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn add_midi_automation(
    clip: &mut Element,
    channel: u8,
    controllers: &BTreeMap<u8, BTreeMap<u8, Vec<ControllerEvent>>>,
    pitch_bends: &BTreeMap<u8, Vec<PitchBendEvent>>,
    pitch_bend_ranges: &BTreeMap<u8, Vec<PitchBendRangeEvent>>,
    pitch_bend_range: f64,
    targets: &BTreeMap<u8, String>,
    ppq: u16,
    length: f64,
    write_cc: bool,
    write_pb: bool,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let mut next_envelope_id = next_clip_envelope_id(clip);

    if write_cc && let Some(channel_controllers) = controllers.get(&channel) {
        for (controller, events) in channel_controllers {
            // ControllerTargets.0 is Pitch Bend; CC n is ControllerTargets.(n+1).
            let target_index = controller.saturating_add(1);
            let Some(target_id) = targets.get(&target_index) else {
                if *controller == 64 {
                    warnings.push(format!(
                        "通道 A{:02} 的轨道没有 ControllerTargets.{target_index}，CC64 未写入。",
                        channel + 1
                    ));
                }
                continue;
            };
            let Some(points) = controller_points(events, ppq, length) else {
                continue;
            };
            let envelope = build_clip_envelope(next_envelope_id, target_id, &points);
            next_envelope_id += 1;
            push_clip_envelope(clip, envelope);
        }
    }

    if write_pb
        && let Some(events) = pitch_bends.get(&channel)
        && let Some(points) = pitch_bend_points(
            events,
            pitch_bend_ranges
                .get(&channel)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            ppq,
            length,
            pitch_bend_range,
        )
    {
        let Some(target_id) = targets.get(&0) else {
            warnings.push(format!(
                "通道 A{:02} 的轨道没有 ControllerTargets.0（Pitch Bend），Pitch Bend 未写入。",
                channel + 1
            ));
            return Ok(());
        };
        let envelope = build_clip_envelope(next_envelope_id, target_id, &points);
        push_clip_envelope(clip, envelope);
    }

    Ok(())
}

pub(crate) fn push_clip_envelope(clip: &mut Element, envelope: Element) {
    let envelopes = ensure_child(clip, "Envelopes");
    let inner = ensure_child(envelopes, "Envelopes");
    inner.children.push(XMLNode::Element(envelope));
}

pub(crate) fn pitch_bend_points(
    events: &[PitchBendEvent],
    ranges: &[PitchBendRangeEvent],
    ppq: u16,
    length: f64,
    fixed_range: f64,
) -> Option<Vec<(f64, f64)>> {
    if !fixed_range.is_finite() || fixed_range <= 0.0 {
        return None;
    }

    let mut by_tick: BTreeMap<u64, u16> = BTreeMap::new();
    for event in events {
        by_tick.insert(event.tick, event.value);
    }
    let mut ranges = ranges.to_vec();
    ranges.sort_by_key(|event| event.tick);

    let mut range_index = 0usize;
    let mut current_range = 2.0; // Standard MIDI default: +/- 2 semitones.
    let mut initial = 0.0;
    let mut changes = Vec::new();
    for (tick, value) in by_tick {
        while range_index < ranges.len() && ranges[range_index].tick <= tick {
            current_range = ranges[range_index].semitones;
            range_index += 1;
        }
        let beat = tick as f64 / ppq as f64;
        if beat > length {
            continue;
        }
        let signed = value as f64 - 8192.0;
        let scaled = (signed * current_range / fixed_range).clamp(-8192.0, 8191.0);
        if beat == 0.0 {
            initial = scaled;
        } else {
            changes.push((beat, scaled));
        }
    }

    if changes.is_empty() {
        return if initial == 0.0 {
            None
        } else {
            Some(vec![(-63_072_000.0, initial)])
        };
    }

    let mut points = vec![(-63_072_000.0, initial), (0.0, initial)];
    let mut current = initial;
    for (beat, value) in changes {
        if value == current {
            continue;
        }
        points.push((beat, value));
        current = value;
    }
    if points.len() <= 2 {
        None
    } else {
        Some(points)
    }
}

pub(crate) fn controller_points(
    events: &[ControllerEvent],
    ppq: u16,
    length: f64,
) -> Option<Vec<(f64, f64)>> {
    let mut by_tick: BTreeMap<u64, u8> = BTreeMap::new();
    for event in events {
        by_tick.insert(event.tick, event.value);
    }

    let mut initial = 0.0;
    let mut changes = Vec::new();
    for (tick, value) in by_tick {
        let beat = tick as f64 / ppq as f64;
        if beat > length {
            continue;
        }
        if beat == 0.0 {
            initial = value as f64;
        } else {
            changes.push((beat, value as f64));
        }
    }

    if changes.is_empty() {
        return if initial == 0.0 {
            None
        } else {
            Some(vec![(-63_072_000.0, initial)])
        };
    }

    let mut points = vec![(-63_072_000.0, initial)];
    let mut current = initial;
    for (beat, value) in changes {
        if value == current {
            continue;
        }
        points.push((beat, current));
        points.push((beat, value));
        current = value;
    }
    if points.len() <= 1 {
        None
    } else {
        Some(points)
    }
}

pub(crate) fn next_clip_envelope_id(clip: &Element) -> u64 {
    clip.get_child("Envelopes")
        .and_then(|outer| outer.get_child("Envelopes"))
        .into_iter()
        .flat_map(|inner| inner.children.iter())
        .filter_map(XMLNode::as_element)
        .filter(|element| element.name == "ClipEnvelope")
        .filter_map(|element| attr(element, "Id"))
        .filter_map(|id| id.parse::<u64>().ok())
        .max()
        .map(|max| max + 1)
        .unwrap_or(0)
}

pub(crate) fn build_clip_envelope(
    envelope_id: u64,
    target_id: &str,
    points: &[(f64, f64)],
) -> Element {
    let mut envelope = Element::new("ClipEnvelope");
    envelope
        .attributes
        .insert("Id".into(), envelope_id.to_string());

    let mut target = Element::new("EnvelopeTarget");
    let mut pointee = Element::new("PointeeId");
    pointee
        .attributes
        .insert("Value".into(), target_id.to_string());
    target.children.push(XMLNode::Element(pointee));
    envelope.children.push(XMLNode::Element(target));

    let mut automation = Element::new("Automation");
    let mut events = Element::new("Events");
    for (index, (time, value)) in points.iter().enumerate() {
        let mut event = Element::new("FloatEvent");
        event
            .attributes
            .insert("Id".into(), (index + 1).to_string());
        event
            .attributes
            .insert("Time".into(), crate::format_num(*time));
        event
            .attributes
            .insert("Value".into(), crate::format_num(*value));
        events.children.push(XMLNode::Element(event));
    }
    automation.children.push(XMLNode::Element(events));

    let mut view_state = Element::new("AutomationTransformViewState");
    let mut pending = Element::new("IsTransformPending");
    pending.attributes.insert("Value".into(), "false".into());
    view_state.children.push(XMLNode::Element(pending));
    view_state
        .children
        .push(XMLNode::Element(Element::new("TimeAndValueTransforms")));
    automation.children.push(XMLNode::Element(view_state));
    envelope.children.push(XMLNode::Element(automation));

    let mut loop_slot = Element::new("LoopSlot");
    loop_slot
        .children
        .push(XMLNode::Element(Element::new("Value")));
    envelope.children.push(XMLNode::Element(loop_slot));

    let mut scroller = Element::new("ScrollerTimePreserver");
    let mut left = Element::new("LeftTime");
    left.attributes.insert("Value".into(), "0".into());
    let mut right = Element::new("RightTime");
    right.attributes.insert("Value".into(), "0".into());
    scroller.children.push(XMLNode::Element(left));
    scroller.children.push(XMLNode::Element(right));
    envelope.children.push(XMLNode::Element(scroller));

    envelope
}
