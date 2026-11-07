use super::*;

pub(crate) fn set_track_name(track: &mut Element, name: &str) {
    if let Some(name_el) = track.get_mut_child("Name") {
        set_child_value(name_el, "EffectiveName", name);
        if name_el.get_child("UserName").is_some() {
            set_child_value(name_el, "UserName", name);
        }
    }
}

pub(crate) fn ensure_arranger_events(track: &mut Element) -> Result<&mut Element> {
    let arranger = find_descendant_mut(track, "ArrangerAutomation").context(
        "目标轨道缺少 ArrangerAutomation。请在 Live 中把占位 Clip 拖到 Arrangement 后保存模板。",
    )?;
    child_mut(arranger, "Events").context("ArrangerAutomation 缺少 Events 节点")
}

pub(crate) fn track_effect_names(track: &Element) -> Vec<String> {
    const EFFECT_TAGS: &[&str] = &[
        "Reverb",
        "HybridReverb",
        "Delay",
        "FilterDelay",
        "Echo",
        "Chorus2",
    ];
    fn walk(element: &Element, names: &mut Vec<String>) {
        if EFFECT_TAGS.contains(&element.name.as_str()) && !names.contains(&element.name) {
            names.push(element.name.clone());
        }
        for child in element.children.iter().filter_map(XMLNode::as_element) {
            walk(child, names);
        }
    }
    let mut names = Vec::new();
    walk(track, &mut names);
    names
}

pub(crate) fn bypass_instrument_effects(track: &mut Element) {
    const EFFECT_TAGS: &[&str] = &[
        "Reverb",
        "HybridReverb",
        "Delay",
        "FilterDelay",
        "Echo",
        "Chorus2",
    ];
    fn walk(element: &mut Element) {
        if EFFECT_TAGS.contains(&element.name.as_str()) {
            let on_tag = if element.get_child("On").is_some() {
                "On"
            } else {
                "DeviceOn"
            };
            if let Some(on) = element.get_mut_child(on_tag)
                && let Some(manual) = on.get_mut_child("Manual")
            {
                manual.attributes.insert("Value".into(), "false".into());
            }
        }
        for child in element
            .children
            .iter_mut()
            .filter_map(XMLNode::as_mut_element)
        {
            walk(child);
        }
    }
    walk(track);
}

/// Set Simpler/Sampler/Operator-style PitchBendRange parameters in a cloned
/// instrument. Returns false when the device does not expose a writable range.
pub(crate) fn set_pitch_bend_range(track: &mut Element, semitones: f64) -> bool {
    fn walk(element: &mut Element, value: &str) -> bool {
        let mut found = false;
        for child in element
            .children
            .iter_mut()
            .filter_map(XMLNode::as_mut_element)
        {
            if child.name.eq_ignore_ascii_case("PitchBendRange") {
                child.attributes.insert("Value".into(), value.into());
                found = true;
            }
            found |= walk(child, value);
        }
        found
    }
    walk(track, &crate::format_num(semitones))
}

pub(crate) fn track_devices(track: &Element) -> (Vec<String>, bool) {
    let Some(outer) = track.get_child("DeviceChain") else {
        return (Vec::new(), false);
    };
    let Some(inner) = outer.get_child("DeviceChain") else {
        return (Vec::new(), false);
    };
    let Some(devices) = inner.get_child("Devices") else {
        return (Vec::new(), false);
    };

    let mut names = Vec::new();
    let mut has_instrument = false;
    for device in devices.children.iter().filter_map(XMLNode::as_element) {
        has_instrument |= INSTRUMENT_TAGS.contains(&device.name.as_str());
        let user_name = find_descendant(device, "UserName")
            .and_then(|e| attr(e, "Value"))
            .filter(|s| !s.is_empty());
        if let Some(name) = user_name {
            names.push(format!("{}: {}", device.name, name));
        } else {
            names.push(device.name.clone());
        }
    }
    (names, has_instrument)
}

pub(crate) fn looks_like_piano(track_name: &str, devices: &[String]) -> bool {
    if track_name.to_lowercase().contains("piano") {
        return true;
    }
    devices
        .iter()
        .any(|device| device.to_lowercase().contains("piano"))
}

pub(crate) fn group_children_valid(tracks: &[TrackInfo], group_id: u32) -> bool {
    let children: Vec<&TrackInfo> = tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Midi && track.group_id == Some(group_id))
        .collect();
    if children.len() == 1 {
        return true;
    }
    if children.len() != 2 {
        return false;
    }
    let instruments: Vec<&TrackInfo> = children
        .iter()
        .copied()
        .filter(|track| track.has_instrument)
        .collect();
    let midis: Vec<&TrackInfo> = children
        .iter()
        .copied()
        .filter(|track| !track.has_instrument)
        .collect();
    instruments.len() == 1
        && midis.len() == 1
        && midis[0]
            .midi_target
            .is_some_and(|target| target == instruments[0].id.unwrap_or(u32::MAX))
}

pub(crate) fn is_channel_group_name(name: &str) -> bool {
    name.strip_prefix('A')
        .is_some_and(|rest| rest.len() == 2 && rest.chars().all(|ch| ch.is_ascii_digit()))
}

pub(crate) fn track_kind(track: &Element) -> TrackKind {
    match track.name.as_str() {
        "MidiTrack" => TrackKind::Midi,
        "GroupTrack" => TrackKind::Group,
        "ReturnTrack" => TrackKind::Return,
        _ => TrackKind::Other,
    }
}

pub(crate) fn track_id(track: &Element) -> Option<u32> {
    attr(track, "Id").and_then(|id| id.parse().ok())
}

pub(crate) fn track_group_id(track: &Element) -> Option<u32> {
    track
        .get_child("TrackGroupId")
        .and_then(|e| attr(e, "Value"))
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value >= 0)
        .map(|value| value as u32)
}

pub(crate) fn track_name(track: &Element) -> String {
    track
        .get_child("Name")
        .and_then(|name| {
            name.get_child("EffectiveName")
                .or_else(|| name.get_child("UserName"))
        })
        .and_then(|e| attr(e, "Value"))
        .unwrap_or("(未命名轨道)")
        .to_string()
}
