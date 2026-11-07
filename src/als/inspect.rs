use super::*;
use std::path::Path;

pub fn inspect_template(path: &Path) -> Result<TemplateReport> {
    let xml = read_als_xml(path)?;
    let root = parse_xml(&xml)?;
    let clips = enumerate_clips(&root)?;
    build_report(path, &root, &clips)
}

pub fn inspect_template_bytes(label: &str, raw: &[u8]) -> Result<TemplateReport> {
    let xml = read_als_bytes(raw, label)?;
    let root = parse_xml(&xml)?;
    let clips = enumerate_clips(&root)?;
    build_report(Path::new(label), &root, &clips)
}

pub(crate) fn enumerate_clips(root: &Element) -> Result<Vec<RawClip>> {
    let live_set = live_set(root).context("模板里找不到 <LiveSet> 节点")?;
    let tracks = live_set
        .get_child("Tracks")
        .context("模板里找不到 <LiveSet>/<Tracks> 节点")?;

    let mut group_names: HashMap<u32, String> = HashMap::new();
    let mut group_element_indices: HashMap<u32, usize> = HashMap::new();
    for (element_index, node) in tracks.children.iter().enumerate() {
        let Some(track) = node.as_element() else {
            continue;
        };
        if track_kind(track) == TrackKind::Group
            && let Some(id) = track_id(track)
        {
            group_names.insert(id, track_name(track));
            group_element_indices.insert(id, element_index);
        }
    }

    let mut clips = Vec::new();
    let mut track_index = 0usize;
    for (track_element_index, node) in tracks.children.iter().enumerate() {
        let Some(track) = node.as_element() else {
            continue;
        };
        let kind = track_kind(track);
        if kind == TrackKind::Other {
            continue;
        }
        let current_track_index = track_index;
        track_index += 1;

        if kind != TrackKind::Midi {
            continue;
        }
        let track_name = track_name(track);
        let group_id = track_group_id(track);
        let group_name = group_id.and_then(|id| group_names.get(&id).cloned());
        let group_element_index = group_id.and_then(|id| group_element_indices.get(&id).copied());

        walk_clips(
            track,
            track_element_index,
            current_track_index,
            &track_name,
            group_id,
            group_name.as_deref(),
            group_element_index,
            &mut Vec::new(),
            ClipContext::Other,
            &mut clips,
        );
    }

    // Arrangement first, then Session; track order and XML order inside each group.
    clips.sort_by(|a, b| {
        location_order(a.info.location)
            .cmp(&location_order(b.info.location))
            .then_with(|| a.info.track_index.cmp(&b.info.track_index))
            .then_with(|| a.info.time.total_cmp(&b.info.time))
            .then_with(|| a.info.name.cmp(&b.info.name))
    });
    for (i, clip) in clips.iter_mut().enumerate() {
        clip.info.candidate_index = i;
    }
    Ok(clips)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClipContext {
    Arrangement,
    Session,
    Other,
}

pub(crate) fn location_order(location: ClipLocation) -> u8 {
    match location {
        ClipLocation::Arrangement => 0,
        ClipLocation::Session => 1,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn walk_clips(
    element: &Element,
    track_element_index: usize,
    track_index: usize,
    track_name: &str,
    group_id: Option<u32>,
    group_name: Option<&str>,
    group_element_index: Option<usize>,
    path: &mut Vec<usize>,
    context: ClipContext,
    out: &mut Vec<RawClip>,
) {
    for (i, node) in element.children.iter().enumerate() {
        let Some(child) = node.as_element() else {
            continue;
        };
        let child_context = match child.name.as_str() {
            "ArrangerAutomation" => ClipContext::Arrangement,
            "ClipSlotList" => ClipContext::Session,
            _ => context,
        };

        if child.name == "MidiClip" {
            let location = match child_context {
                ClipContext::Arrangement => Some(ClipLocation::Arrangement),
                ClipContext::Session => Some(ClipLocation::Session),
                ClipContext::Other => None,
            };
            if let Some(location) = location {
                let mut clip_path = path.clone();
                clip_path.push(i);
                let mut info = clip_info(child, location, track_index, track_name, 0);
                info.group_id = group_id;
                info.group_name = group_name.map(str::to_string);
                out.push(RawClip {
                    info,
                    track_element_index,
                    clip_path,
                    group_id,
                    group_element_index,
                });
            }
            continue;
        }

        path.push(i);
        walk_clips(
            child,
            track_element_index,
            track_index,
            track_name,
            group_id,
            group_name,
            group_element_index,
            path,
            child_context,
            out,
        );
        path.pop();
    }
}

pub(crate) fn clip_info(
    clip: &Element,
    location: ClipLocation,
    track_index: usize,
    track_name: &str,
    candidate_index: usize,
) -> ClipInfo {
    let loop_el = clip.get_child("Loop");
    let loop_start = loop_el
        .map(|e| child_f64(e, "LoopStart"))
        .unwrap_or_default();
    let loop_end = loop_el.map(|e| child_f64(e, "LoopEnd")).unwrap_or_default();
    let loop_on = loop_el
        .and_then(|e| e.get_child("LoopOn"))
        .and_then(|e| e.attributes.get("Value"))
        .is_some_and(|v| v == "true");

    ClipInfo {
        candidate_index,
        track_index,
        track_name: track_name.to_string(),
        group_id: None,
        group_name: None,
        location,
        name: clip
            .get_child("Name")
            .and_then(|e| attr(e, "Value"))
            .unwrap_or("(未命名 Clip)")
            .to_string(),
        time: attr_f64(clip, "Time").unwrap_or(0.0),
        current_start: child_f64(clip, "CurrentStart"),
        current_end: child_f64(clip, "CurrentEnd"),
        loop_start,
        loop_end,
        loop_on,
        note_count: count_descendants(clip, "MidiNoteEvent"),
        key_track_count: count_descendants(clip, "KeyTrack"),
    }
}

pub(crate) fn build_report(
    path: &Path,
    root: &Element,
    clips: &[RawClip],
) -> Result<TemplateReport> {
    let live_set = live_set(root).context("模板里找不到 <LiveSet> 节点")?;
    let tracks_el = live_set
        .get_child("Tracks")
        .context("模板里找不到 <LiveSet>/<Tracks> 节点")?;

    let mut group_names: HashMap<u32, String> = HashMap::new();
    for node in &tracks_el.children {
        let Some(track) = node.as_element() else {
            continue;
        };
        if track_kind(track) == TrackKind::Group
            && let Some(id) = track_id(track)
        {
            group_names.insert(id, track_name(track));
        }
    }

    let mut tracks = Vec::new();
    let mut midi_targets = Vec::new();
    let mut track_index = 0usize;
    let mut midi_track_count = 0usize;
    let mut group_count = 0usize;
    for node in &tracks_el.children {
        let Some(track) = node.as_element() else {
            continue;
        };
        let kind = track_kind(track);
        if kind == TrackKind::Other {
            continue;
        }
        let group_id = track_group_id(track);
        let group_name = group_id.and_then(|id| group_names.get(&id).cloned());
        let (devices, has_instrument) = track_devices(track);
        tracks.push(TrackInfo {
            index: track_index,
            id: track_id(track),
            name: track_name(track),
            kind,
            group_id,
            group_name,
            devices,
            has_instrument,
            midi_target: None,
            routed_to_instrument: false,
        });
        midi_targets.push(midi_routed_track_id(track));
        match kind {
            TrackKind::Midi => midi_track_count += 1,
            TrackKind::Group => group_count += 1,
            TrackKind::Return | TrackKind::Other => {}
        }
        track_index += 1;
    }
    let instrument_ids: HashSet<u32> = tracks
        .iter()
        .filter(|track| track.has_instrument)
        .filter_map(|track| track.id)
        .collect();
    for (track, target) in tracks.iter_mut().zip(midi_targets) {
        track.midi_target = target;
        track.routed_to_instrument =
            target.is_some_and(|target_id| instrument_ids.contains(&target_id));
    }

    let bpm = read_tempo(live_set);
    let recommended_clip = clips
        .iter()
        .position(|c| c.info.group_id.is_some() && c.info.location == ClipLocation::Arrangement)
        .or_else(|| clips.iter().position(|c| c.info.group_id.is_some()))
        .or_else(|| {
            clips
                .iter()
                .position(|c| c.info.location == ClipLocation::Arrangement)
        })
        .or(if clips.is_empty() { None } else { Some(0) });

    let mut warnings = Vec::new();
    if midi_track_count == 0 {
        warnings.push("模板里没有 MIDI 轨道。".into());
    }
    if group_count == 0 {
        warnings.push(
            "模板没有 Group。请创建一个 Group（例如 A01），把加载 Grand Piano 的 MIDI 轨道和占位 Clip 放进这个 Group。"
                .into(),
        );
    } else if group_count > 1 {
        warnings.push(format!(
            "模板有 {} 个 Group；转换器只会克隆所选 Clip 所在的 Group。",
            group_count
        ));
    }
    if clips.is_empty() {
        warnings.push("模板里没有 MidiClip。".into());
    } else if clips.len() > 1 {
        warnings.push(format!(
            "模板有 {} 个 MIDI Clip；转换器使用所选 Clip 所在的 Group 作为原型。",
            clips.len()
        ));
    }
    if let Some(recommended) = recommended_clip {
        let clip = &clips[recommended];
        if clip.info.group_id.is_none() {
            warnings.push(
                "所选 Clip 不在 Group 内；当前版本按通道分组，需要把占位 Clip 放进 Group。".into(),
            );
        }
        if let Some(track) = tracks.iter().find(|t| t.index == clip.info.track_index) {
            if track.has_instrument {
                if !looks_like_piano(&track.name, &track.devices) {
                    warnings.push(format!(
                        "目标轨道「{}」没有检测到钢琴类音色；输出会使用模板里的音色。",
                        track.name
                    ));
                }
            } else if let Some(target) = track
                .midi_target
                .and_then(|id| tracks.iter().find(|target| target.id == Some(id)))
            {
                if !looks_like_piano(&target.name, &target.devices) {
                    warnings.push(format!(
                        "目标 MIDI 轨「{}」路由到的乐器轨「{}」没有检测到钢琴类音色。",
                        track.name, target.name
                    ));
                }
            } else {
                warnings.push(format!(
                    "目标轨道「{}」未检测到乐器设备，也没有路由到 Group 内的乐器轨。",
                    track.name
                ));
            }
        }
        if let Some(group_id) = clip.info.group_id
            && !group_children_valid(&tracks, group_id)
        {
            warnings.push(
                "原型 Group 需要 1 条“乐器+Clip”轨，或 1 条乐器轨 + 1 条 MIDI 轨（MIDI To 指向乐器轨）。"
                    .into(),
            );
        }
    }
    for track in &tracks {
        if track.kind != TrackKind::Group || !is_channel_group_name(&track.name) {
            continue;
        }
        if let Some(group_id) = track.id
            && !group_children_valid(&tracks, group_id)
        {
            warnings.push(format!(
                "Group「{}」需要 1 条“乐器+Clip”轨，或 1 条乐器轨 + 1 条 MIDI 轨（MIDI To 指向乐器轨）。",
                track.name
            ));
        }
        if !clips
            .iter()
            .any(|clip| clip.info.group_name.as_deref() == Some(track.name.as_str()))
        {
            warnings.push(format!(
                "Group「{}」没有占位 Clip；对应通道会回退到默认原型。",
                track.name
            ));
        }
    }
    let minor_version = attr(root, "MinorVersion").unwrap_or("?");
    if !minor_version.starts_with("12.") {
        warnings.push(format!(
            "当前只在 Live 12.3 模板上验证；模板版本 {minor_version} 请先用小样测试。"
        ));
    }
    if bpm.is_none() {
        warnings.push("模板里找不到全局 Tempo（MainTrack/DeviceChain/Mixer/Tempo）。".into());
    }

    Ok(TemplateReport {
        path: path.to_path_buf(),
        major_version: attr(root, "MajorVersion").unwrap_or("?").to_string(),
        minor_version: minor_version.to_string(),
        creator: attr(root, "Creator").unwrap_or("?").to_string(),
        revision: attr(root, "Revision").unwrap_or("?").to_string(),
        bpm,
        tracks,
        clips: clips.iter().map(|c| c.info.clone()).collect(),
        recommended_clip,
        warnings,
    })
}

pub(crate) fn select_clip<'a>(
    clips: &'a [RawClip],
    index: Option<usize>,
    name: Option<&str>,
) -> Result<&'a RawClip> {
    if clips.is_empty() {
        bail!("模板里没有可用的 MidiClip；请先在 Live 中创建一个占位 Clip");
    }

    if let Some(index) = index {
        return clips
            .get(index)
            .with_context(|| format!("Clip 序号 {index} 超出范围（共 {} 个 Clip）", clips.len()));
    }

    if let Some(name) = name.map(str::trim).filter(|s| !s.is_empty()) {
        let exact: Vec<&RawClip> = clips
            .iter()
            .filter(|c| c.info.name.eq_ignore_ascii_case(name))
            .collect();
        if exact.len() == 1 {
            return Ok(exact[0]);
        }
        let contains: Vec<&RawClip> = clips
            .iter()
            .filter(|c| c.info.name.to_lowercase().contains(&name.to_lowercase()))
            .collect();
        return match contains.len() {
            1 => Ok(contains[0]),
            0 => bail!(
                "找不到名称匹配「{name}」的 Clip。可用 Clip：{}",
                clip_name_list(clips)
            ),
            _ => bail!(
                "名称「{name}」匹配到多个 Clip（{}）。请用 Clip 序号或改一个唯一的占位 Clip 名称。",
                contains
                    .iter()
                    .map(|c| c.info.display_label())
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        };
    }

    if let Some(clip) = clips
        .iter()
        .find(|c| c.group_id.is_some() && c.info.location == ClipLocation::Arrangement)
    {
        return Ok(clip);
    }
    if let Some(clip) = clips.iter().find(|c| c.group_id.is_some()) {
        return Ok(clip);
    }
    if let Some(clip) = clips
        .iter()
        .find(|c| c.info.location == ClipLocation::Arrangement)
    {
        return Ok(clip);
    }
    Ok(&clips[0])
}

pub(crate) fn clip_name_list(clips: &[RawClip]) -> String {
    clips
        .iter()
        .map(|c| {
            format!(
                "#{}「{}」({})",
                c.info.candidate_index, c.info.name, c.info.location
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}
