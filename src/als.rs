use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use flate2::read::MultiGzDecoder;
use flate2::{Compression, GzBuilder};
use xmltree::{Element, EmitterConfig, XMLNode};

use crate::midi::{ControllerEvent, Note, PitchBendEvent};

const INSTRUMENT_TAGS: &[&str] = &[
    "InstrumentGroupDevice",
    "OriginalSimpler",
    "Simpler",
    "Sampler",
    "DrumGroupDevice",
    "DrumRack",
    "Meld",
    "Drift",
    "Operator",
    "Wavetable",
    "Bass",
    "Collision",
    "Electric",
    "Impulse",
    "Tension",
    "Analog",
    "ExternalInstrument",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipLocation {
    Arrangement,
    Session,
}

impl fmt::Display for ClipLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arrangement => f.write_str("Arrangement"),
            Self::Session => f.write_str("Session"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackKind {
    Midi,
    Group,
    Return,
    Other,
}

impl fmt::Display for TrackKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Midi => f.write_str("MIDI"),
            Self::Group => f.write_str("Group"),
            Self::Return => f.write_str("Return"),
            Self::Other => f.write_str("Other"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClipInfo {
    /// Index in `TemplateReport::clips`, stable for the same template.
    pub candidate_index: usize,
    pub track_index: usize,
    pub track_name: String,
    pub group_id: Option<u32>,
    pub group_name: Option<String>,
    pub location: ClipLocation,
    pub name: String,
    pub time: f64,
    pub current_start: f64,
    pub current_end: f64,
    pub loop_start: f64,
    pub loop_end: f64,
    pub loop_on: bool,
    pub note_count: usize,
    pub key_track_count: usize,
}

impl ClipInfo {
    pub fn length_beats(&self) -> f64 {
        let current = self.current_end - self.current_start;
        if current > 0.0 {
            current
        } else {
            (self.loop_end - self.loop_start).max(0.0)
        }
    }

    pub fn display_label(&self) -> String {
        let group = self
            .group_name
            .as_ref()
            .map(|name| format!("Group「{name}」/ "))
            .unwrap_or_default();
        format!(
            "{}{} / 轨道「{}」/ Clip「{}」（{} 音符, {} beats）",
            group,
            self.location,
            self.track_name,
            self.name,
            self.note_count,
            crate::format_num(self.length_beats())
        )
    }
}

#[derive(Clone, Debug)]
pub struct TrackInfo {
    pub index: usize,
    pub id: Option<u32>,
    pub name: String,
    pub kind: TrackKind,
    pub group_id: Option<u32>,
    pub group_name: Option<String>,
    pub devices: Vec<String>,
    pub has_instrument: bool,
    pub midi_target: Option<u32>,
    pub routed_to_instrument: bool,
}

#[derive(Clone, Debug)]
pub struct TemplateReport {
    pub path: PathBuf,
    pub major_version: String,
    pub minor_version: String,
    pub creator: String,
    pub revision: String,
    pub bpm: Option<f64>,
    pub tracks: Vec<TrackInfo>,
    pub clips: Vec<ClipInfo>,
    /// First Arrangement clip if present, otherwise the first Session clip.
    pub recommended_clip: Option<usize>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct RawClip {
    pub info: ClipInfo,
    pub track_element_index: usize,
    /// Raw child indices from the track element to the `<MidiClip>`.
    pub clip_path: Vec<usize>,
    pub group_id: Option<u32>,
    pub group_element_index: Option<usize>,
}

pub(crate) struct ChildPlan {
    pub source_track: u16,
    pub track_name: String,
    pub clip_name: String,
    pub notes: Vec<Note>,
    pub clip_length_beats: f64,
}

pub(crate) struct GroupPlan {
    pub channel: u8,
    pub group_name: String,
    pub prototype: RawClip,
    pub children: Vec<ChildPlan>,
}

pub(crate) struct GroupInjectRequest<'a> {
    pub groups: &'a [GroupPlan],
    pub clip_start_beats: f64,
    pub bpm: f64,
    pub time_signature: Option<(u8, u16)>,
    /// (arrangement beat, BPM) pairs. `None` writes a constant tempo only.
    pub tempo_map: Option<&'a [(f64, f64)]>,
    /// channel -> controller number -> absolute-tick events.
    pub controllers: &'a BTreeMap<u8, BTreeMap<u8, Vec<ControllerEvent>>>,
    /// channel -> raw 14-bit pitch bend events.
    pub pitch_bends: &'a BTreeMap<u8, Vec<PitchBendEvent>>,
    pub ppq: u16,
    pub write_pitch_bend: bool,
    pub write_cc_automation: bool,
    pub strip_instrument_effects: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct InjectionInfo {
    pub tempo_automation_written: bool,
    pub warnings: Vec<String>,
}

/// Read an `.als` file and return the decompressed XML bytes.
///
/// Both gzip-compressed `.als` and plain XML are accepted, which makes debugging
/// a hand-extracted template easier.
pub fn read_als_xml(path: &Path) -> Result<Vec<u8>> {
    let raw = std::fs::read(path).with_context(|| format!("读取 .als 失败: {}", path.display()))?;
    read_als_bytes(&raw, &path.display().to_string())
}

/// Decompress an `.als` byte slice. Plain XML is accepted for debugging.
pub fn read_als_bytes(raw: &[u8], label: &str) -> Result<Vec<u8>> {
    if raw.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = MultiGzDecoder::new(raw);
        let mut xml = Vec::new();
        decoder
            .read_to_end(&mut xml)
            .with_context(|| format!("解压 gzip 失败: {label}"))?;
        if xml.is_empty() {
            bail!(".als 解压后是空文件: {label}");
        }
        Ok(xml)
    } else {
        if raw.is_empty() {
            bail!(".als 是空文件: {label}");
        }
        Ok(raw.to_vec())
    }
}

/// Write the decompressed XML next to the user for manual inspection.
pub fn dump_template_xml(template: &Path, output: &Path) -> Result<()> {
    let xml = read_als_xml(template)?;
    write_xml_file(output, &xml)
}

pub fn dump_als_bytes_xml(label: &str, raw: &[u8], output: &Path) -> Result<()> {
    let xml = read_als_bytes(raw, label)?;
    write_xml_file(output, &xml)
}

fn write_xml_file(output: &Path, xml: &[u8]) -> Result<()> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, xml).with_context(|| format!("写入 XML 失败: {}", output.display()))?;
    Ok(())
}

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

#[inline]
pub(crate) fn parse_xml(xml: &[u8]) -> Result<Element> {
    Element::parse(Cursor::new(xml)).context("模板 XML 解析失败：请确认它是有效的 Ableton .als")
}

pub(crate) fn serialize_xml(root: &Element) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(1024 * 1024);
    out.extend_from_slice(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    root.write_with_config(
        &mut out,
        EmitterConfig::new().write_document_declaration(false),
    )
    .context("序列化 Ableton XML 失败")?;
    Ok(out)
}

/// Compress XML and atomically replace the destination `.als`.
pub(crate) fn write_als_atomic(output: &Path, xml: &[u8]) -> Result<()> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::default());
    encoder.write_all(xml).context("gzip 压缩 XML 失败")?;
    let gz = encoder.finish().context("完成 gzip 压缩失败")?;

    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .with_context(|| format!("创建输出目录失败: {}", parent.display()))?;
    let file_name = output
        .file_name()
        .context("输出路径没有文件名")?
        .to_string_lossy();
    let tmp = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));

    std::fs::write(&tmp, &gz).with_context(|| format!("写入临时文件失败: {}", tmp.display()))?;
    if let Err(err) = std::fs::rename(&tmp, output) {
        let _ = std::fs::remove_file(&tmp);
        return Err(err).with_context(|| format!("替换输出文件失败: {}", output.display()));
    }
    Ok(())
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
enum ClipContext {
    Arrangement,
    Session,
    Other,
}

fn location_order(location: ClipLocation) -> u8 {
    match location {
        ClipLocation::Arrangement => 0,
        ClipLocation::Session => 1,
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_clips(
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

fn clip_info(
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

fn build_report(path: &Path, root: &Element, clips: &[RawClip]) -> Result<TemplateReport> {
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

fn clip_name_list(clips: &[RawClip]) -> String {
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

pub(crate) fn validate_live_set(root: &Element) -> Result<()> {
    validate_sibling_ids(root)?;

    let mut seen = HashSet::new();
    let mut max_pointee = 0u64;
    collect_pointee_ids_for_validation(root, &mut seen, &mut max_pointee)?;

    if let Some(next) = live_set(root)
        .and_then(|live_set| live_set.get_child("NextPointeeId"))
        .and_then(|node| attr(node, "Value"))
        .and_then(|value| value.parse::<u64>().ok())
        && max_pointee >= next
    {
        bail!(
            "Pointee ID 越界：最大 ID {} 不小于 NextPointeeId {}",
            max_pointee,
            next
        );
    }
    Ok(())
}

fn validate_sibling_ids(element: &Element) -> Result<()> {
    let mut seen = HashSet::new();
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        if let Some(id) = attr(child, "Id").and_then(|value| value.parse::<u64>().ok())
            && !seen.insert(id)
        {
            bail!("同一父节点下出现重复 Id：<{} Id=\"{}\">", child.name, id);
        }
    }
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        validate_sibling_ids(child)?;
    }
    Ok(())
}

fn collect_pointee_ids_for_validation(
    element: &Element,
    seen: &mut HashSet<u64>,
    max: &mut u64,
) -> Result<()> {
    if is_pointee_holder(element)
        && let Some(id) = attr(element, "Id").and_then(|value| value.parse::<u64>().ok())
    {
        if !seen.insert(id) {
            bail!("全局 Pointee/AutomationTarget Id 重复：{id}（Live 会拒绝加载）");
        }
        *max = (*max).max(id);
    }
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        collect_pointee_ids_for_validation(child, seen, max)?;
    }
    Ok(())
}

enum PrototypeMode {
    Single { child: Element },
    Routed { instrument: Element, midi: Element },
}

fn resolve_prototype(
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

fn midi_routed_track_id(track: &Element) -> Option<u32> {
    let target = track
        .get_child("DeviceChain")?
        .get_child("MidiOutputRouting")?
        .get_child("Target")
        .and_then(|node| attr(node, "Value"))?;
    let rest = target.strip_prefix("MidiOut/Track.")?;
    rest.split('/').next()?.parse().ok()
}

fn set_midi_output_to_track(track: &mut Element, instrument_id: u32, instrument_name: &str) {
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

fn set_track_group_id(track: &mut Element, value: i64) {
    set_child_value(track, "TrackGroupId", &value.to_string());
}

fn set_linked_track_group_id(track: &mut Element, value: i64) {
    set_child_value(track, "LinkedTrackGroupId", &value.to_string());
}

fn set_track_unfolded(track: &mut Element, unfolded: bool) {
    set_child_value(
        track,
        "TrackUnfolded",
        if unfolded { "true" } else { "false" },
    );
}

fn set_group_output(track: &mut Element) {
    if let Some(routing) = find_descendant_mut(track, "AudioOutputRouting") {
        set_child_value(routing, "Target", "AudioOut/GroupTrack");
        set_child_value(routing, "UpperDisplayString", "Group");
    }
}

fn set_record_arm(track: &mut Element, armed: bool) {
    if let Some(recorder) = find_descendant_mut(track, "Recorder")
        && recorder.get_child("IsArmed").is_some()
    {
        set_child_value(recorder, "IsArmed", if armed { "true" } else { "false" });
    }
}

fn max_track_id(tracks: &Element) -> u32 {
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

fn strip_midi_clips(element: &mut Element) {
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
fn clear_track_devices(track: &mut Element) {
    if let Some(devices) = track
        .get_mut_child("DeviceChain")
        .and_then(|chain| chain.get_mut_child("DeviceChain"))
        .and_then(|chain| chain.get_mut_child("Devices"))
    {
        devices.children.clear();
    }
}

struct PointeeAllocator {
    next: u64,
}

impl PointeeAllocator {
    fn new(live_set: &Element) -> Self {
        let mut max = live_set
            .get_child("NextPointeeId")
            .and_then(|node| attr(node, "Value"))
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(1);
        collect_max_pointee_id(live_set, &mut max);
        Self { next: max + 1 }
    }

    fn allocate(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        id
    }
}

fn collect_max_pointee_id(element: &Element, max: &mut u64) {
    if is_pointee_holder(element)
        && let Some(id) = attr(element, "Id").and_then(|value| value.parse::<u64>().ok())
    {
        *max = (*max).max(id);
    }
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        collect_max_pointee_id(child, max);
    }
}

fn is_pointee_holder(element: &Element) -> bool {
    element.name == "Pointee"
        || element.name.ends_with("AutomationTarget")
        || element.name.ends_with("ModulationTarget")
        || element.name.starts_with("ControllerTargets.")
}

fn remap_pointee_ids(element: &mut Element, allocator: &mut PointeeAllocator) {
    let mut remap = HashMap::new();
    collect_pointee_ids(element, allocator, &mut remap);
    rewrite_pointee_ids(element, &remap);
}

fn collect_pointee_ids(
    element: &Element,
    allocator: &mut PointeeAllocator,
    remap: &mut HashMap<u64, u64>,
) {
    if is_pointee_holder(element)
        && let Some(id) = attr(element, "Id").and_then(|value| value.parse::<u64>().ok())
    {
        remap.entry(id).or_insert_with(|| allocator.allocate());
    }
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        collect_pointee_ids(child, allocator, remap);
    }
}

fn rewrite_pointee_ids(element: &mut Element, remap: &HashMap<u64, u64>) {
    if is_pointee_holder(element)
        && let Some(id) = attr(element, "Id").and_then(|value| value.parse::<u64>().ok())
        && let Some(new_id) = remap.get(&id)
    {
        element.attributes.insert("Id".into(), new_id.to_string());
    }
    if element.name == "PointeeId"
        && let Some(id) = attr(element, "Value").and_then(|value| value.parse::<u64>().ok())
        && let Some(new_id) = remap.get(&id)
    {
        element
            .attributes
            .insert("Value".into(), new_id.to_string());
    }
    for child in element
        .children
        .iter_mut()
        .filter_map(XMLNode::as_mut_element)
    {
        rewrite_pointee_ids(child, remap);
    }
}

fn edit_clip(
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

fn replace_notes(clip: &mut Element, notes: &[Note], write_note_ids: bool) -> Result<()> {
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

fn build_keytracks(notes: &[Note], write_note_ids: bool) -> Element {
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

fn clear_clip_envelopes(clip: &mut Element) {
    if let Some(envelopes) = clip.get_mut_child("Envelopes")
        && let Some(inner) = envelopes.get_mut_child("Envelopes")
    {
        inner.children.clear();
    }
}

fn controller_targets(track: &Element) -> BTreeMap<u8, String> {
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
fn add_midi_automation(
    clip: &mut Element,
    channel: u8,
    controllers: &BTreeMap<u8, BTreeMap<u8, Vec<ControllerEvent>>>,
    pitch_bends: &BTreeMap<u8, Vec<PitchBendEvent>>,
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
        && let Some(points) = pitch_bend_points(events, ppq, length)
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

fn push_clip_envelope(clip: &mut Element, envelope: Element) {
    let envelopes = ensure_child(clip, "Envelopes");
    let inner = ensure_child(envelopes, "Envelopes");
    inner.children.push(XMLNode::Element(envelope));
}

fn pitch_bend_points(events: &[PitchBendEvent], ppq: u16, length: f64) -> Option<Vec<(f64, f64)>> {
    let mut by_tick: BTreeMap<u64, u16> = BTreeMap::new();
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
        let signed = value as i32 - 8192;
        if beat == 0.0 {
            initial = signed as f64;
        } else {
            changes.push((beat, signed as f64));
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

fn controller_points(events: &[ControllerEvent], ppq: u16, length: f64) -> Option<Vec<(f64, f64)>> {
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

fn next_clip_envelope_id(clip: &Element) -> u64 {
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

fn build_clip_envelope(envelope_id: u64, target_id: &str, points: &[(f64, f64)]) -> Element {
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

fn set_tempo(
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

fn build_tempo_events(initial_bpm: f64, tempo_map: Option<&[(f64, f64)]>) -> Vec<(f64, f64)> {
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

fn build_tempo_envelope(
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

fn ensure_automation_envelopes(master: &mut Element) -> &mut Element {
    let automation = ensure_child(master, "AutomationEnvelopes");
    ensure_child(automation, "Envelopes")
}

fn envelope_target_id(envelope: &Element) -> Option<&str> {
    envelope
        .get_child("EnvelopeTarget")?
        .get_child("PointeeId")
        .and_then(|e| attr(e, "Value"))
}

fn next_envelope_id(envelopes: &Element) -> u64 {
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

fn next_float_event_id(master: &Element) -> u64 {
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

fn read_tempo(live_set: &Element) -> Option<f64> {
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

fn set_track_name(track: &mut Element, name: &str) {
    if let Some(name_el) = track.get_mut_child("Name") {
        set_child_value(name_el, "EffectiveName", name);
        if name_el.get_child("UserName").is_some() {
            set_child_value(name_el, "UserName", name);
        }
    }
}

fn ensure_arranger_events(track: &mut Element) -> Result<&mut Element> {
    let arranger = find_descendant_mut(track, "ArrangerAutomation").context(
        "目标轨道缺少 ArrangerAutomation。请在 Live 中把占位 Clip 拖到 Arrangement 后保存模板。",
    )?;
    child_mut(arranger, "Events").context("ArrangerAutomation 缺少 Events 节点")
}

fn track_effect_names(track: &Element) -> Vec<String> {
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

fn bypass_instrument_effects(track: &mut Element) {
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

fn track_devices(track: &Element) -> (Vec<String>, bool) {
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

fn looks_like_piano(track_name: &str, devices: &[String]) -> bool {
    if track_name.to_lowercase().contains("piano") {
        return true;
    }
    devices
        .iter()
        .any(|device| device.to_lowercase().contains("piano"))
}

fn group_children_valid(tracks: &[TrackInfo], group_id: u32) -> bool {
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

fn is_channel_group_name(name: &str) -> bool {
    name.strip_prefix('A')
        .is_some_and(|rest| rest.len() == 2 && rest.chars().all(|ch| ch.is_ascii_digit()))
}

fn track_kind(track: &Element) -> TrackKind {
    match track.name.as_str() {
        "MidiTrack" => TrackKind::Midi,
        "GroupTrack" => TrackKind::Group,
        "ReturnTrack" => TrackKind::Return,
        _ => TrackKind::Other,
    }
}

fn track_id(track: &Element) -> Option<u32> {
    attr(track, "Id").and_then(|id| id.parse().ok())
}

fn track_group_id(track: &Element) -> Option<u32> {
    track
        .get_child("TrackGroupId")
        .and_then(|e| attr(e, "Value"))
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value >= 0)
        .map(|value| value as u32)
}

fn track_name(track: &Element) -> String {
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

fn live_set(root: &Element) -> Option<&Element> {
    if root.name == "LiveSet" {
        Some(root)
    } else {
        root.get_child("LiveSet")
    }
}

fn live_set_mut(root: &mut Element) -> Option<&mut Element> {
    if root.name == "LiveSet" {
        Some(root)
    } else {
        root.get_mut_child("LiveSet")
    }
}

fn count_descendants(element: &Element, name: &str) -> usize {
    let mut count = usize::from(element.name == name);
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        count += count_descendants(child, name);
    }
    count
}

fn find_descendant<'a>(element: &'a Element, name: &str) -> Option<&'a Element> {
    if element.name == name {
        return Some(element);
    }
    for child in element.children.iter().filter_map(XMLNode::as_element) {
        if let Some(found) = find_descendant(child, name) {
            return Some(found);
        }
    }
    None
}

fn find_descendant_mut<'a>(element: &'a mut Element, name: &str) -> Option<&'a mut Element> {
    if element.name == name {
        return Some(element);
    }
    for child in element
        .children
        .iter_mut()
        .filter_map(XMLNode::as_mut_element)
    {
        if let Some(found) = find_descendant_mut(child, name) {
            return Some(found);
        }
    }
    None
}

fn navigate<'a>(mut element: &'a Element, path: &[usize]) -> Option<&'a Element> {
    for &index in path {
        element = element.children.get(index)?.as_element()?;
    }
    Some(element)
}

fn child_mut<'a>(element: &'a mut Element, name: &str) -> Option<&'a mut Element> {
    element
        .children
        .iter_mut()
        .filter_map(XMLNode::as_mut_element)
        .find(|e| e.name == name)
}

fn ensure_child<'a>(element: &'a mut Element, name: &str) -> &'a mut Element {
    let pos = element
        .children
        .iter()
        .position(|node| node.as_element().is_some_and(|e| e.name == name));
    if let Some(pos) = pos {
        return element.children[pos]
            .as_mut_element()
            .expect("checked element");
    }
    element.children.push(XMLNode::Element(Element::new(name)));
    element
        .children
        .last_mut()
        .and_then(XMLNode::as_mut_element)
        .expect("just pushed")
}

fn set_child_value(parent: &mut Element, name: &str, value: &str) {
    let child = ensure_child(parent, name);
    child.attributes.insert("Value".into(), value.into());
}

fn set_attr(element: &mut Element, name: &str, value: &str) {
    element.attributes.insert(name.into(), value.into());
}

fn attr<'a>(element: &'a Element, name: &str) -> Option<&'a str> {
    element.attributes.get(name).map(String::as_str)
}

fn attr_f64(element: &Element, name: &str) -> Option<f64> {
    attr(element, name).and_then(|v| v.parse().ok())
}

fn child_f64(element: &Element, name: &str) -> f64 {
    element
        .get_child(name)
        .and_then(|e| attr_f64(e, "Value"))
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINI_GROUP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Ableton MajorVersion="5" MinorVersion="12.0_12300" Creator="test" Revision="x">
 <LiveSet>
  <NextPointeeId Value="500" />
  <MainTrack><DeviceChain><Mixer>
   <Tempo><Manual Value="120"/><MidiControllerRange><Min Value="60"/><Max Value="200"/></MidiControllerRange><AutomationTarget Id="8"><LockEnvelope Value="0"/></AutomationTarget></Tempo>
  </Mixer></DeviceChain></MainTrack>
  <Tracks>
   <GroupTrack Id="20">
    <Name><EffectiveName Value="A01"/><UserName Value=""/></Name>
    <Color Value="1" />
    <AutomationEnvelopes><Envelopes /></AutomationEnvelopes>
    <TrackGroupId Value="-1" />
    <TrackUnfolded Value="true" />
    <Slots><GroupTrackSlot Id="0"><LomId Value="0" /></GroupTrackSlot></Slots>
    <DeviceChain>
     <Mixer><Pointee Id="300" /></Mixer>
     <DeviceChain><Devices><AudioEffectGroupDevice Id="1"><UserName Value="Group FX" /></AudioEffectGroupDevice></Devices></DeviceChain>
     <AudioOutputRouting><Target Value="AudioOut/Main" /></AudioOutputRouting>
    </DeviceChain>
   </GroupTrack>
   <MidiTrack Id="1">
    <Name><EffectiveName Value="Piano"/><UserName Value=""/></Name>
    <TrackGroupId Value="20" />
    <TrackUnfolded Value="true" />
    <DeviceChain>
     <DeviceChain><Devices><InstrumentGroupDevice Id="2"><UserName Value="Grand Piano" /></InstrumentGroupDevice></Devices></DeviceChain>
     <Mixer><Pointee Id="400" /></Mixer>
     <MidiControllers>
      <ControllerTargets.65 Id="900"><LockEnvelope Value="0" /></ControllerTargets.65>
      <ControllerTargets.0 Id="898"><LockEnvelope Value="0" /></ControllerTargets.0>
     </MidiControllers>
    </DeviceChain>
    <ClipSlotList>
      <ClipSlot Id="0"><ClipSlot><Value /></ClipSlot></ClipSlot>
    </ClipSlotList>
    <ClipTimeable><ArrangerAutomation><Events>
      <MidiClip Id="0" Time="0">
       <CurrentStart Value="0"/><CurrentEnd Value="4"/>
       <Loop><LoopStart Value="0"/><LoopEnd Value="4"/><StartRelative Value="0"/><LoopOn Value="true"/><OutMarker Value="4"/><HiddenLoopStart Value="0"/><HiddenLoopEnd Value="4"/></Loop>
       <Name Value="BLACKLIVE"/>
       <TimeSignature><TimeSignatures><RemoteableTimeSignature Id="0"><Numerator Value="4"/><Denominator Value="4"/><Time Value="0"/></RemoteableTimeSignature></TimeSignatures></TimeSignature>
       <Notes>
        <KeyTracks><KeyTrack Id="0"><Notes><MidiNoteEvent Time="0" Duration="1" Velocity="100" OffVelocity="64" NoteId="1"/></Notes><MidiKey Value="60"/></KeyTrack></KeyTracks>
        <PerNoteEventStore><EventLists /></PerNoteEventStore>
        <NoteProbabilityGroups />
        <ProbabilityGroupIdGenerator><NextId Value="1"/></ProbabilityGroupIdGenerator>
        <NoteIdGenerator><NextId Value="2"/></NoteIdGenerator>
       </Notes>
      </MidiClip>
    </Events></ArrangerAutomation></ClipTimeable>
   </MidiTrack>
  </Tracks>
 </LiveSet>
</Ableton>"#;

    fn notes() -> Vec<Note> {
        vec![
            Note {
                key: 60,
                start: 0.0,
                duration: 1.0,
                velocity: 100.0,
                off_velocity: 64.0,
                channel: 0,
                source_track: 0,
            },
            Note {
                key: 64,
                start: 2.0,
                duration: 0.5,
                velocity: 80.0,
                off_velocity: 64.0,
                channel: 0,
                source_track: 0,
            },
        ]
    }

    fn controllers() -> BTreeMap<u8, BTreeMap<u8, Vec<ControllerEvent>>> {
        let mut controllers = BTreeMap::new();
        controllers.entry(0).or_insert_with(BTreeMap::new).insert(
            64,
            vec![
                ControllerEvent { tick: 0, value: 0 },
                ControllerEvent {
                    tick: 480,
                    value: 127,
                },
                ControllerEvent {
                    tick: 960,
                    value: 0,
                },
            ],
        );
        controllers
    }

    fn pitch_bends() -> BTreeMap<u8, Vec<PitchBendEvent>> {
        let mut bends = BTreeMap::new();
        bends.insert(
            0,
            vec![
                PitchBendEvent {
                    tick: 0,
                    value: 8192,
                },
                PitchBendEvent {
                    tick: 480,
                    value: 16383,
                },
                PitchBendEvent {
                    tick: 960,
                    value: 8192,
                },
            ],
        );
        bends
    }

    fn single_group_plan(prototype: RawClip) -> Vec<GroupPlan> {
        vec![GroupPlan {
            channel: 0,
            group_name: "A01".to_string(),
            prototype,
            children: vec![ChildPlan {
                source_track: 0,
                track_name: "Piano".to_string(),
                clip_name: "Test".to_string(),
                notes: notes(),
                clip_length_beats: 4.0,
            }],
        }]
    }

    #[test]
    fn inspect_and_inject_group_clip() {
        let mut root = parse_xml(MINI_GROUP.as_bytes()).unwrap();
        let clips = enumerate_clips(&root).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].info.group_name.as_deref(), Some("A01"));
        assert_eq!(clips[0].info.note_count, 1);

        let groups = single_group_plan(clips[0].clone());
        let info = inject_groups(
            &mut root,
            GroupInjectRequest {
                groups: &groups,
                clip_start_beats: 0.0,
                bpm: 90.0,
                time_signature: Some((4, 4)),
                tempo_map: None,
                controllers: &controllers(),
                pitch_bends: &pitch_bends(),
                ppq: 480,
                write_pitch_bend: true,
                write_cc_automation: true,
                strip_instrument_effects: true,
            },
        )
        .unwrap();
        assert!(!info.tempo_automation_written);

        validate_live_set(&root).unwrap();

        let xml = String::from_utf8(serialize_xml(&root).unwrap()).unwrap();
        assert!(xml.contains("Velocity=\"100\""));
        assert!(xml.contains("MidiKey Value=\"64\""));
        assert!(xml.contains("Manual Value=\"90\""));
        assert_eq!(xml.matches("<GroupTrack ").count(), 1);
        assert_eq!(xml.matches("<MidiTrack").count(), 1);
        assert_eq!(xml.matches("<MidiNoteEvent").count(), 2);
        assert_eq!(xml.matches("<ClipEnvelope").count(), 2);
        assert!(xml.contains("ControllerTargets.0"));
        assert!(xml.contains("Value=\"8191\""));
    }

    #[test]
    fn multiple_children_get_unique_ids_and_group_links() {
        let mut root = parse_xml(MINI_GROUP.as_bytes()).unwrap();
        let clips = enumerate_clips(&root).unwrap();
        let groups = vec![GroupPlan {
            channel: 0,
            group_name: "A01".to_string(),
            prototype: clips[0].clone(),
            children: vec![
                ChildPlan {
                    source_track: 0,
                    track_name: "Piano A".to_string(),
                    clip_name: "A".to_string(),
                    notes: notes(),
                    clip_length_beats: 4.0,
                },
                ChildPlan {
                    source_track: 1,
                    track_name: "Piano B".to_string(),
                    clip_name: "B".to_string(),
                    notes: notes(),
                    clip_length_beats: 4.0,
                },
            ],
        }];
        inject_groups(
            &mut root,
            GroupInjectRequest {
                groups: &groups,
                clip_start_beats: 0.0,
                bpm: 120.0,
                time_signature: Some((4, 4)),
                tempo_map: None,
                controllers: &controllers(),
                pitch_bends: &pitch_bends(),
                ppq: 480,
                write_pitch_bend: true,
                write_cc_automation: true,
                strip_instrument_effects: true,
            },
        )
        .unwrap();

        validate_live_set(&root).unwrap();

        let xml = String::from_utf8(serialize_xml(&root).unwrap()).unwrap();
        assert_eq!(xml.matches("<GroupTrack ").count(), 1);
        assert_eq!(xml.matches("<MidiTrack").count(), 2);
        assert_eq!(xml.matches("<MidiNoteEvent").count(), 4);
        assert_eq!(xml.matches("<ClipEnvelope").count(), 4);
        assert!(xml.contains("Piano A"));
        assert!(xml.contains("Piano B"));
        assert_eq!(xml.matches("<TrackGroupId Value=\"").count(), 3);
    }

    #[test]
    fn single_mode_empty_child_keeps_track_without_instrument() {
        let mut root = parse_xml(MINI_GROUP.as_bytes()).unwrap();
        let clips = enumerate_clips(&root).unwrap();
        let groups = vec![GroupPlan {
            channel: 0,
            group_name: "A01".to_string(),
            prototype: clips[0].clone(),
            children: vec![
                ChildPlan {
                    source_track: 0,
                    track_name: "Piano A".to_string(),
                    clip_name: "A".to_string(),
                    notes: notes(),
                    clip_length_beats: 4.0,
                },
                ChildPlan {
                    source_track: 1,
                    track_name: "Empty".to_string(),
                    clip_name: "Empty".to_string(),
                    notes: Vec::new(),
                    clip_length_beats: 0.0,
                },
            ],
        }];
        inject_groups(
            &mut root,
            GroupInjectRequest {
                groups: &groups,
                clip_start_beats: 0.0,
                bpm: 120.0,
                time_signature: Some((4, 4)),
                tempo_map: None,
                controllers: &controllers(),
                pitch_bends: &pitch_bends(),
                ppq: 480,
                write_pitch_bend: true,
                write_cc_automation: false,
                strip_instrument_effects: true,
            },
        )
        .unwrap();

        validate_live_set(&root).unwrap();

        let xml = String::from_utf8(serialize_xml(&root).unwrap()).unwrap();
        assert_eq!(xml.matches("<GroupTrack ").count(), 1);
        assert_eq!(xml.matches("<MidiTrack").count(), 2);
        assert_eq!(xml.matches("<InstrumentGroupDevice").count(), 1);
        assert_eq!(xml.matches("<MidiClip").count(), 1);
        assert_eq!(xml.matches("<MidiNoteEvent").count(), 2);
        assert!(xml.contains("Empty"));
    }

    #[test]
    fn embedded_default_template_passes_live_id_validation() {
        let xml =
            read_als_bytes(crate::DEFAULT_TEMPLATE_BYTES, crate::DEFAULT_TEMPLATE_LABEL).unwrap();
        let root = parse_xml(&xml).unwrap();
        validate_live_set(&root).unwrap();
    }

    #[test]
    fn freeze_sequencer_modulation_targets_get_unique_ids() {
        let mut root = parse_xml(MINI_GROUP.as_bytes()).unwrap();
        {
            let live_set = live_set_mut(&mut root).unwrap();
            let tracks = live_set.get_mut_child("Tracks").unwrap();
            let track = tracks
                .children
                .iter_mut()
                .filter_map(XMLNode::as_mut_element)
                .find(|element| element.name == "MidiTrack" && attr(element, "Id") == Some("1"))
                .unwrap();
            let chain = track.get_mut_child("DeviceChain").unwrap();
            let mut freeze = Element::new("FreezeSequencer");
            for (tag, id) in [
                ("VolumeModulationTarget", "777"),
                ("GrainSizeModulationTarget", "778"),
            ] {
                let mut target = Element::new(tag);
                target.attributes.insert("Id".into(), id.into());
                freeze.children.push(XMLNode::Element(target));
            }
            chain.children.push(XMLNode::Element(freeze));
        }

        let clips = enumerate_clips(&root).unwrap();
        let groups = vec![GroupPlan {
            channel: 0,
            group_name: "A01".to_string(),
            prototype: clips[0].clone(),
            children: vec![
                ChildPlan {
                    source_track: 0,
                    track_name: "Piano A".to_string(),
                    clip_name: "A".to_string(),
                    notes: notes(),
                    clip_length_beats: 4.0,
                },
                ChildPlan {
                    source_track: 1,
                    track_name: "Piano B".to_string(),
                    clip_name: "B".to_string(),
                    notes: notes(),
                    clip_length_beats: 4.0,
                },
            ],
        }];
        inject_groups(
            &mut root,
            GroupInjectRequest {
                groups: &groups,
                clip_start_beats: 0.0,
                bpm: 120.0,
                time_signature: Some((4, 4)),
                tempo_map: None,
                controllers: &controllers(),
                pitch_bends: &pitch_bends(),
                ppq: 480,
                write_pitch_bend: true,
                write_cc_automation: false,
                strip_instrument_effects: true,
            },
        )
        .unwrap();

        validate_live_set(&root).unwrap();

        let mut ids = Vec::new();
        fn walk(element: &Element, ids: &mut Vec<String>) {
            if matches!(
                element.name.as_str(),
                "VolumeModulationTarget" | "GrainSizeModulationTarget"
            ) && let Some(id) = attr(element, "Id")
            {
                ids.push(id.to_string());
            }
            for child in element.children.iter().filter_map(XMLNode::as_element) {
                walk(child, ids);
            }
        }
        walk(&root, &mut ids);
        assert_eq!(ids.len(), 4);
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 4);
    }

    #[test]
    fn tempo_map_writes_step_automation() {
        let mut root = parse_xml(MINI_GROUP.as_bytes()).unwrap();
        let clips = enumerate_clips(&root).unwrap();
        let groups = single_group_plan(clips[0].clone());
        let tempo_map = [(0.0, 60.0), (0.5, 137.0), (4.0, 120.0)];
        let info = inject_groups(
            &mut root,
            GroupInjectRequest {
                groups: &groups,
                clip_start_beats: 0.0,
                bpm: 60.0,
                time_signature: Some((4, 4)),
                tempo_map: Some(&tempo_map),
                controllers: &controllers(),
                pitch_bends: &pitch_bends(),
                ppq: 480,
                write_pitch_bend: false,
                write_cc_automation: false,
                strip_instrument_effects: true,
            },
        )
        .unwrap();
        assert!(info.tempo_automation_written);

        validate_live_set(&root).unwrap();

        let xml = String::from_utf8(serialize_xml(&root).unwrap()).unwrap();
        assert!(xml.contains("<AutomationEnvelope"));
        assert!(xml.contains("<PointeeId Value=\"8\""));
        assert!(xml.contains("Time=\"0.5\" Value=\"60\""));
        assert!(xml.contains("Time=\"0.5\" Value=\"137\""));
        assert!(xml.contains("Time=\"4\" Value=\"120\""));
    }
}
