use std::fmt;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use flate2::read::MultiGzDecoder;
use flate2::{Compression, GzBuilder};
use xmltree::{Element, EmitterConfig, XMLNode};

use crate::midi::Note;

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

#[derive(Clone, Debug)]
pub struct ClipInfo {
    /// Index in `TemplateReport::clips`, stable for the same template.
    pub candidate_index: usize,
    pub track_index: usize,
    pub track_name: String,
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
        format!(
            "{} / 轨道「{}」/ Clip「{}」（{} 音符, {} beats）",
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
    pub name: String,
    pub devices: Vec<String>,
    pub has_instrument: bool,
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
}

pub(crate) struct InjectRequest<'a> {
    pub notes: &'a [Note],
    pub clip_length_beats: f64,
    pub clip_start_beats: f64,
    pub bpm: f64,
    pub clip_name: &'a str,
    pub track_name: Option<&'a str>,
    pub time_signature: Option<(u8, u16)>,
    /// (arrangement beat, BPM) pairs. `None` writes a constant tempo only.
    pub tempo_map: Option<&'a [(f64, f64)]>,
}

#[derive(Clone, Debug)]
pub(crate) struct InjectionInfo {
    pub arrangement_created: bool,
    pub template_label: String,
    pub tempo_automation_written: bool,
    #[allow(dead_code)]
    pub notes_in_clip: usize,
    pub warnings: Vec<String>,
}

/// Read an `.als` file and return the decompressed XML bytes.
///
/// Both gzip-compressed `.als` and plain XML are accepted, which makes debugging
/// a hand-extracted template easier.
pub fn read_als_xml(path: &Path) -> Result<Vec<u8>> {
    let raw = std::fs::read(path).with_context(|| format!("读取 .als 失败: {}", path.display()))?;
    if raw.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = MultiGzDecoder::new(raw.as_slice());
        let mut xml = Vec::new();
        decoder
            .read_to_end(&mut xml)
            .with_context(|| format!("解压 gzip 失败: {}", path.display()))?;
        if xml.is_empty() {
            bail!(".als 解压后是空文件: {}", path.display());
        }
        Ok(xml)
    } else {
        if raw.is_empty() {
            bail!(".als 是空文件: {}", path.display());
        }
        Ok(raw)
    }
}

/// Write the decompressed XML next to the user for manual inspection.
pub fn dump_template_xml(template: &Path, output: &Path) -> Result<()> {
    let xml = read_als_xml(template)?;
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

    let mut clips = Vec::new();
    let mut midi_track_index = 0usize;
    for (track_element_index, node) in tracks.children.iter().enumerate() {
        let Some(track) = node.as_element() else {
            continue;
        };
        if track.name != "MidiTrack" {
            continue;
        }
        let track_name = track_name(track);
        let track_index = midi_track_index;
        midi_track_index += 1;

        walk_clips(
            track,
            track_element_index,
            track_index,
            &track_name,
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
                out.push(RawClip {
                    info: clip_info(child, location, track_index, track_name, 0),
                    track_element_index,
                    clip_path,
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

    let mut tracks = Vec::new();
    let mut midi_track_index = 0usize;
    let mut non_midi_tracks = 0usize;
    for node in &tracks_el.children {
        let Some(track) = node.as_element() else {
            continue;
        };
        if track.name != "MidiTrack" {
            non_midi_tracks += 1;
            continue;
        }
        let (devices, has_instrument) = track_devices(track);
        tracks.push(TrackInfo {
            index: midi_track_index,
            name: track_name(track),
            devices,
            has_instrument,
        });
        midi_track_index += 1;
    }

    let bpm = read_tempo(live_set);
    let recommended_clip = clips
        .iter()
        .position(|c| c.info.location == ClipLocation::Arrangement)
        .or(if clips.is_empty() { None } else { Some(0) });

    let mut warnings = Vec::new();
    if tracks.is_empty() {
        warnings.push("模板里没有 MIDI 轨道。请创建一条加载 Grand Piano 的 MIDI 轨道。".into());
    } else if tracks.len() > 1 {
        warnings.push(format!(
            "模板有 {} 条 MIDI 轨道；转换器只会修改所选 Clip 所在的轨道，其他轨道会原样保留。",
            tracks.len()
        ));
    }
    if clips.is_empty() {
        warnings.push("模板里没有 MidiClip。请在目标轨道上放一个短的占位 Clip 并保存。".into());
    } else if clips.len() > 1 {
        warnings.push(format!(
            "模板有 {} 个 MIDI Clip；建议使用只含一个 Clip 的干净模板，或准确选择要替换的 Clip。",
            clips.len()
        ));
    }
    if !clips.is_empty()
        && !clips
            .iter()
            .any(|c| c.info.location == ClipLocation::Arrangement)
    {
        warnings.push(
            "模板没有 Arrangement Clip；转换器会把所选 Session Clip 克隆到 Arrangement，打开工程后按空格即可播放。"
                .into(),
        );
    }
    if non_midi_tracks > 0 {
        warnings.push(format!(
            "模板还包含 {} 条非 MIDI 轨道（音频/Return/Master 等），它们会原样保留。",
            non_midi_tracks
        ));
    }
    if let Some(recommended) = recommended_clip {
        let clip = &clips[recommended];
        if let Some(track) = tracks.iter().find(|t| t.index == clip.info.track_index) {
            if !track.has_instrument {
                warnings.push(format!(
                    "目标轨道「{}」未检测到乐器设备。请确认它加载的是 Grand Piano。",
                    track.name
                ));
            } else if !looks_like_grand_piano(&track.name, &track.devices) {
                warnings.push(format!(
                    "目标轨道「{}」没有检测到名为 Grand Piano 的设备；如果模板用的是其他音色，输出会使用它。",
                    track.name
                ));
            }
        }
    }
    if bpm.is_none() {
        warnings.push("模板里找不到全局 Tempo（MainTrack/DeviceChain/Mixer/Tempo）。".into());
    }

    Ok(TemplateReport {
        path: path.to_path_buf(),
        major_version: attr(root, "MajorVersion").unwrap_or("?").to_string(),
        minor_version: attr(root, "MinorVersion").unwrap_or("?").to_string(),
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

pub(crate) fn inject_notes(
    root: &mut Element,
    clip: &RawClip,
    req: InjectRequest<'_>,
) -> Result<InjectionInfo> {
    if !req.clip_length_beats.is_finite() || req.clip_length_beats <= 0.0 {
        bail!("内部错误：Clip 长度必须是正数");
    }
    if !req.clip_start_beats.is_finite() || req.clip_start_beats < 0.0 {
        bail!("内部错误：Clip 起点必须是非负数");
    }
    if !req.bpm.is_finite() || req.bpm <= 0.0 {
        bail!("内部错误：BPM 必须是正数");
    }

    let mut warnings = Vec::new();
    let live_set = live_set_mut(root).context("模板里找不到 <LiveSet> 节点")?;

    let tempo_automation_written = set_tempo(live_set, req.bpm, req.tempo_map, &mut warnings);
    let tracks = child_mut(live_set, "Tracks").context("模板里找不到 <LiveSet>/<Tracks> 节点")?;
    let track = mutate_navigate(tracks, &[clip.track_element_index])
        .context("无法定位模板中的目标 MIDI 轨道")?;
    let display_track_name = req
        .track_name
        .map(str::to_string)
        .unwrap_or_else(|| clip.info.track_name.clone());

    let (devices, has_instrument) = track_devices(track);
    if !has_instrument {
        warnings.push(format!(
            "目标轨道「{}」没有检测到乐器设备（设备：{}）。请确认模板加载了 Grand Piano。",
            display_track_name,
            if devices.is_empty() {
                "无".to_string()
            } else {
                devices.join(", ")
            }
        ));
    } else if !looks_like_grand_piano(&display_track_name, &devices) {
        warnings.push(format!(
            "目标轨道「{}」没有检测到名为 Grand Piano 的设备（设备：{}）；输出会使用模板里的音色。",
            display_track_name,
            devices.join(", ")
        ));
    }

    if clip.info.location == ClipLocation::Session
        && track_has_clip_in_context(track, ClipContext::Arrangement)
    {
        bail!(
            "所选 Clip 在 Session，但同一轨道已经有 Arrangement Clip。\
             这样会生成两个同时播放的 Clip。请选 Arrangement 里的占位 Clip，\
             或删掉 Arrangement Clip 后重新保存模板。"
        );
    }

    if let Some(track_name) = req.track_name {
        set_track_name(track, track_name);
    }

    {
        let clip_el =
            mutate_navigate(track, &clip.clip_path).context("无法定位模板中的目标 MidiClip")?;
        edit_clip(
            clip_el,
            clip.info.location == ClipLocation::Arrangement,
            &req,
        )?;
    }

    let mut arrangement_created = false;
    if clip.info.location == ClipLocation::Session {
        let source = navigate(track, &clip.clip_path)
            .context("克隆前定位 Session Clip 失败")?
            .clone();
        let events = ensure_arranger_events(track)?;
        let new_index = events.children.len();
        events.children.push(XMLNode::Element(source));
        let new_clip = events.children[new_index]
            .as_mut_element()
            .context("克隆 Arrangement Clip 失败")?;

        // The Session clip uses clip-relative current start/end. The Arrangement
        // clone must use arrangement timeline coordinates instead.
        set_attr(new_clip, "Time", &crate::format_num(req.clip_start_beats));
        set_child_value(
            new_clip,
            "CurrentStart",
            &crate::format_num(req.clip_start_beats),
        );
        set_child_value(
            new_clip,
            "CurrentEnd",
            &crate::format_num(req.clip_start_beats + req.clip_length_beats),
        );
        arrangement_created = true;
        warnings.push(
            "模板 Clip 位于 Session；已同时写入 Arrangement，打开工程后按空格即可播放。".into(),
        );
    } else if track_has_clip_in_context(track, ClipContext::Session) {
        warnings.push(
            "目标轨道还保留着 Session Clip（占位音符未修改）。输出工程使用 Arrangement 播放；\
             如果也会从 Session 启动，请先检查那个 Clip。"
                .into(),
        );
    }

    let target_clip = if arrangement_created {
        find_descendant(track, "ArrangerAutomation")
            .and_then(|a| a.get_child("Events"))
            .and_then(|events| {
                events
                    .children
                    .iter()
                    .filter_map(XMLNode::as_element)
                    .rev()
                    .find(|e| e.name == "MidiClip")
            })
            .context("写入后找不到新的 Arrangement Clip")?
    } else {
        navigate(track, &clip.clip_path).context("写入后找不到目标 Clip")?
    };

    let notes_in_clip = count_descendants(target_clip, "MidiNoteEvent");
    if notes_in_clip != req.notes.len() {
        bail!(
            "内部校验失败：写入了 {} 个音符，但 Clip 中读到 {} 个",
            req.notes.len(),
            notes_in_clip
        );
    }

    Ok(InjectionInfo {
        arrangement_created,
        template_label: clip.info.display_label(),
        tempo_automation_written,
        notes_in_clip,
        warnings,
    })
}

fn edit_clip(clip: &mut Element, arrangement: bool, req: &InjectRequest<'_>) -> Result<()> {
    let start = req.clip_start_beats;
    let length = req.clip_length_beats;

    set_attr(
        clip,
        "Time",
        &crate::format_num(if arrangement { start } else { 0.0 }),
    );
    if arrangement {
        set_child_value(clip, "CurrentStart", &crate::format_num(start));
        set_child_value(clip, "CurrentEnd", &crate::format_num(start + length));
    } else {
        set_child_value(clip, "CurrentStart", "0");
        set_child_value(clip, "CurrentEnd", &crate::format_num(length));
    }

    let loop_el = ensure_child(clip, "Loop");
    set_child_value(loop_el, "LoopStart", "0");
    set_child_value(loop_el, "LoopEnd", &crate::format_num(length));
    set_child_value(loop_el, "StartRelative", "0");
    set_child_value(loop_el, "LoopOn", "false");
    set_child_value(loop_el, "OutMarker", &crate::format_num(length));
    set_child_value(loop_el, "HiddenLoopStart", "0");
    set_child_value(loop_el, "HiddenLoopEnd", &crate::format_num(length));

    set_child_value(clip, "Name", req.clip_name);
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

    if let Some((numerator, denominator)) = req.time_signature
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

    replace_notes(clip, req.notes)
}

fn replace_notes(clip: &mut Element, notes: &[Note]) -> Result<()> {
    let notes_el = clip
        .get_mut_child("Notes")
        .context("模板 Clip 没有 <Notes> 节点，无法注入音符")?;

    let keytracks = build_keytracks(notes);
    if let Some(existing) = notes_el.get_mut_child("KeyTracks") {
        *existing = keytracks;
    } else {
        notes_el.children.insert(0, XMLNode::Element(keytracks));
    }

    // Clear per-note data that referenced the template's placeholder notes.
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

fn build_keytracks(notes: &[Note]) -> Element {
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
            event
                .attributes
                .insert("NoteId".into(), note_id.to_string());
            note_id += 1;
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

fn track_has_clip_in_context(track: &Element, wanted: ClipContext) -> bool {
    fn walk(el: &Element, context: ClipContext, wanted: ClipContext) -> bool {
        for child in el.children.iter().filter_map(XMLNode::as_element) {
            let child_context = match child.name.as_str() {
                "ArrangerAutomation" => ClipContext::Arrangement,
                "ClipSlotList" => ClipContext::Session,
                _ => context,
            };
            if child.name == "MidiClip" && child_context == wanted {
                return true;
            }
            if walk(child, child_context, wanted) {
                return true;
            }
        }
        false
    }
    walk(track, ClipContext::Other, wanted)
}

fn ensure_arranger_events(track: &mut Element) -> Result<&mut Element> {
    let arranger = find_descendant_mut(track, "ArrangerAutomation").context(
        "目标轨道缺少 ArrangerAutomation。请在 Live 中把占位 Clip 拖到 Arrangement 后保存模板。",
    )?;
    child_mut(arranger, "Events").context("ArrangerAutomation 缺少 Events 节点")
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

fn looks_like_grand_piano(track_name: &str, devices: &[String]) -> bool {
    let track = track_name.to_lowercase();
    if track.contains("grand piano") || track.contains("grandpiano") {
        return true;
    }
    devices.iter().any(|device| {
        let device = device.to_lowercase();
        device.contains("grand piano") || device.contains("grandpiano")
    })
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

fn mutate_navigate<'a>(mut element: &'a mut Element, path: &[usize]) -> Option<&'a mut Element> {
    for &index in path {
        element = element.children.get_mut(index)?.as_mut_element()?;
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

    const MINI: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Ableton MajorVersion="5" MinorVersion="12.0_12300" Creator="test" Revision="x">
 <LiveSet>
  <MainTrack><DeviceChain><Mixer>
   <Tempo><Manual Value="120"/><MidiControllerRange><Min Value="60"/><Max Value="200"/></MidiControllerRange><AutomationTarget Id="8"><LockEnvelope Value="0"/></AutomationTarget></Tempo>
  </Mixer></DeviceChain></MainTrack>
  <Tracks>
   <MidiTrack Id="1">
    <Name><EffectiveName Value="Piano"/><UserName Value=""/></Name>
    <DeviceChain><DeviceChain><Devices>
      <InstrumentGroupDevice Id="2"><UserName Value="Grand Piano"/></InstrumentGroupDevice>
    </Devices></DeviceChain></DeviceChain>
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
            },
            Note {
                key: 64,
                start: 2.0,
                duration: 0.5,
                velocity: 80.0,
                off_velocity: 64.0,
                channel: 0,
            },
        ]
    }

    #[test]
    fn inspect_and_inject_arrangement_clip() {
        let root = parse_xml(MINI.as_bytes()).unwrap();
        let clips = enumerate_clips(&root).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].info.location, ClipLocation::Arrangement);
        assert_eq!(clips[0].info.note_count, 1);

        let mut root = root;
        let clip = clips[0].clone();
        let info = inject_notes(
            &mut root,
            &clip,
            InjectRequest {
                notes: &notes(),
                clip_length_beats: 4.0,
                clip_start_beats: 0.0,
                bpm: 90.0,
                clip_name: "Test",
                track_name: Some("Piano"),
                time_signature: Some((4, 4)),
                tempo_map: None,
            },
        )
        .unwrap();
        assert_eq!(info.notes_in_clip, 2);
        assert!(!info.arrangement_created);

        let xml = String::from_utf8(serialize_xml(&root).unwrap()).unwrap();
        assert!(xml.contains("Velocity=\"100\""));
        assert!(xml.contains("MidiKey Value=\"64\""));
        assert!(xml.contains("Manual Value=\"90\""));
        assert!(xml.contains("NoteIdGenerator><NextId Value=\"3\""));
    }

    #[test]
    fn tempo_map_writes_step_automation() {
        let mut root = parse_xml(MINI.as_bytes()).unwrap();
        let clips = enumerate_clips(&root).unwrap();
        let clip = clips[0].clone();
        let tempo_map = [(0.0, 60.0), (0.5, 137.0), (4.0, 120.0)];
        let info = inject_notes(
            &mut root,
            &clip,
            InjectRequest {
                notes: &notes(),
                clip_length_beats: 4.0,
                clip_start_beats: 0.0,
                bpm: 60.0,
                clip_name: "Test",
                track_name: Some("Piano"),
                time_signature: Some((4, 4)),
                tempo_map: Some(&tempo_map),
            },
        )
        .unwrap();
        assert!(info.tempo_automation_written);

        let xml = String::from_utf8(serialize_xml(&root).unwrap()).unwrap();
        assert!(xml.contains("<AutomationEnvelope"));
        assert!(xml.contains("<PointeeId Value=\"8\""));
        assert!(xml.contains("Time=\"0.5\" Value=\"60\""));
        assert!(xml.contains("Time=\"0.5\" Value=\"137\""));
        assert!(xml.contains("Time=\"4\" Value=\"120\""));
    }

    #[test]
    fn session_clip_is_cloned_to_arrangement() {
        let clip_start = MINI.find("<MidiClip").unwrap();
        let clip_end = MINI.find("</MidiClip>").unwrap() + "</MidiClip>".len();
        let clip = &MINI[clip_start..clip_end];

        let events_start = MINI.find("<Events>").unwrap() + "<Events>".len();
        let events_end = MINI.find("</Events>").unwrap();
        let without_arrangement = format!("{}{}", &MINI[..events_start], &MINI[events_end..]);
        let session_xml =
            without_arrangement.replacen("<Value />", &format!("<Value>{clip}</Value>"), 1);

        let mut root = parse_xml(session_xml.as_bytes()).unwrap();
        let clips = enumerate_clips(&root).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].info.location, ClipLocation::Session);
        let clip = clips[0].clone();

        let info = inject_notes(
            &mut root,
            &clip,
            InjectRequest {
                notes: &notes(),
                clip_length_beats: 4.0,
                clip_start_beats: 0.0,
                bpm: 90.0,
                clip_name: "Test",
                track_name: Some("Piano"),
                time_signature: Some((4, 4)),
                tempo_map: None,
            },
        )
        .unwrap();
        assert!(info.arrangement_created);
        assert_eq!(info.notes_in_clip, 2);

        let xml = String::from_utf8(serialize_xml(&root).unwrap()).unwrap();
        // Session source and arrangement clone each contain two notes.
        assert_eq!(xml.matches("<MidiNoteEvent").count(), 4);
        assert_eq!(xml.matches("<ArrangerAutomation>").count(), 1);
    }
}
