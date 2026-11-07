use super::*;

pub(crate) fn set_tempo(
    live_set: &mut Element,
    bpm: f64,
    tempo_map: Option<&[(f64, f64)]>,
    warnings: &mut Vec<String>,
) -> bool {
    let master = if live_set.get_child("MainTrack").is_some() {
        live_set.get_mut_child("MainTrack")
    } else {
        live_set.get_mut_child("MasterTrack")
    };
    let Some(master) = master else {
        warnings.push("模板里找不到 MainTrack/MasterTrack，BPM 未写入。".into());
        return false;
    };

    let target_id = {
        let Some(tempo) = master
            .get_mut_child("DeviceChain")
            .and_then(|dc| dc.get_mut_child("Mixer"))
            .and_then(|mixer| mixer.get_mut_child("Tempo"))
        else {
            warnings.push("模板里找不到 Mixer/Tempo，BPM 未写入。".into());
            return false;
        };

        if let Some(manual) = tempo.get_mut_child("Manual") {
            manual
                .attributes
                .insert("Value".into(), crate::format_num(bpm));
        } else {
            warnings.push("模板 Tempo 节点没有 Manual 子节点，BPM 未写入。".into());
        }

        if let Some(range) = tempo.get_child("MidiControllerRange") {
            let min = child_f64(range, "Min");
            let max = child_f64(range, "Max");
            let mut outside = bpm < min || bpm > max;
            if let Some(map) = tempo_map {
                outside |= map.iter().any(|(_, value)| *value < min || *value > max);
            }
            if outside {
                warnings.push(format!(
                    "BPM 超出模板 Tempo 控件范围 {}–{}；Live 可能会夹紧这些值。",
                    crate::format_num(min),
                    crate::format_num(max)
                ));
            }
        }

        tempo
            .get_child("AutomationTarget")
            .and_then(|e| attr(e, "Id"))
            .map(str::to_string)
    };

    let Some(target_id) = target_id else {
        if tempo_map.is_some() {
            warnings.push(
                "模板 Tempo 没有 AutomationTarget Id，无法写入变速自动化；已只写入固定 BPM。"
                    .into(),
            );
        }
        return false;
    };

    let events = build_tempo_events(bpm, tempo_map);
    let automation_written = events.len() > 1;
    let first_event_id = next_float_event_id(master);

    let envelopes = ensure_automation_envelopes(master);
    let existing = envelopes.children.iter().position(|node| {
        node.as_element()
            .filter(|e| e.name == "AutomationEnvelope")
            .and_then(envelope_target_id)
            .is_some_and(|id| id == target_id)
    });

    if let Some(index) = existing {
        let envelope_id = envelopes.children[index]
            .as_element()
            .and_then(|e| attr(e, "Id"))
            .unwrap_or("0")
            .to_string();
        let envelope = build_tempo_envelope(&envelope_id, &target_id, &events, first_event_id);
        envelopes.children[index] = XMLNode::Element(envelope);
    } else if automation_written {
        let envelope_id = next_envelope_id(envelopes).to_string();
        let envelope = build_tempo_envelope(&envelope_id, &target_id, &events, first_event_id);
        envelopes.children.push(XMLNode::Element(envelope));
    }

    automation_written
}

pub(crate) fn build_tempo_events(
    initial_bpm: f64,
    tempo_map: Option<&[(f64, f64)]>,
) -> Vec<(f64, f64)> {
    // Ableton's tempo automation uses a sentinel event far in the past so the
    // value is defined before the arrangement start.
    let mut events = vec![(-63_072_000.0, initial_bpm)];
    let Some(map) = tempo_map else {
        return events;
    };

    let mut current = initial_bpm;
    for &(beat, bpm) in map {
        if !beat.is_finite() || !bpm.is_finite() || bpm <= 0.0 {
            continue;
        }
        if beat <= 0.0 {
            current = bpm;
            continue;
        }
        if bpm == current {
            continue;
        }
        // Two events at the same beat encode a step: hold the old value up to
        // this point, then jump to the new value. A single event would make
        // Ableton interpolate the tempo ramp.
        events.push((beat, current));
        events.push((beat, bpm));
        current = bpm;
    }
    events
}

pub(crate) fn build_tempo_envelope(
    envelope_id: &str,
    target_id: &str,
    events: &[(f64, f64)],
    first_event_id: u64,
) -> Element {
    let mut envelope = Element::new("AutomationEnvelope");
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
    let mut events_el = Element::new("Events");
    for (index, (time, value)) in events.iter().enumerate() {
        let mut event = Element::new("FloatEvent");
        event
            .attributes
            .insert("Id".into(), (first_event_id + index as u64).to_string());
        event
            .attributes
            .insert("Time".into(), crate::format_num(*time));
        event
            .attributes
            .insert("Value".into(), crate::format_num(*value));
        events_el.children.push(XMLNode::Element(event));
    }
    automation.children.push(XMLNode::Element(events_el));

    let mut view_state = Element::new("AutomationTransformViewState");
    let mut pending = Element::new("IsTransformPending");
    pending.attributes.insert("Value".into(), "false".into());
    view_state.children.push(XMLNode::Element(pending));
    view_state
        .children
        .push(XMLNode::Element(Element::new("TimeAndValueTransforms")));
    automation.children.push(XMLNode::Element(view_state));
    envelope.children.push(XMLNode::Element(automation));

    envelope
}

pub(crate) fn ensure_automation_envelopes(master: &mut Element) -> &mut Element {
    let automation = ensure_child(master, "AutomationEnvelopes");
    ensure_child(automation, "Envelopes")
}

pub(crate) fn envelope_target_id(envelope: &Element) -> Option<&str> {
    envelope
        .get_child("EnvelopeTarget")?
        .get_child("PointeeId")
        .and_then(|e| attr(e, "Value"))
}

pub(crate) fn next_envelope_id(envelopes: &Element) -> u64 {
    envelopes
        .children
        .iter()
        .filter_map(XMLNode::as_element)
        .filter(|e| e.name == "AutomationEnvelope")
        .filter_map(|e| attr(e, "Id"))
        .filter_map(|id| id.parse::<u64>().ok())
        .max()
        .map(|max| max + 1)
        .unwrap_or(0)
}

pub(crate) fn next_float_event_id(master: &Element) -> u64 {
    fn walk(element: &Element, max: &mut u64) {
        for child in element.children.iter().filter_map(XMLNode::as_element) {
            if child.name == "FloatEvent"
                && let Some(id) = attr(child, "Id").and_then(|id| id.parse::<u64>().ok())
            {
                *max = (*max).max(id);
            }
            walk(child, max);
        }
    }
    let mut max = 0u64;
    walk(master, &mut max);
    max + 1
}

pub(crate) fn template_bpm(root: &Element) -> Option<f64> {
    live_set(root).and_then(read_tempo)
}

pub(crate) fn read_tempo(live_set: &Element) -> Option<f64> {
    let master = live_set
        .get_child("MainTrack")
        .or_else(|| live_set.get_child("MasterTrack"))?;
    master
        .get_child("DeviceChain")?
        .get_child("Mixer")?
        .get_child("Tempo")?
        .get_child("Manual")
        .and_then(|e| attr_f64(e, "Value"))
}
