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

fn no_pb_ranges() -> BTreeMap<u8, Vec<PitchBendRangeEvent>> {
    BTreeMap::new()
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
            pitch_bend_ranges: &no_pb_ranges(),
            pitch_bend_range: 2.0,
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
            pitch_bend_ranges: &no_pb_ranges(),
            pitch_bend_range: 2.0,
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
            pitch_bend_ranges: &no_pb_ranges(),
            pitch_bend_range: 2.0,
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
fn pitch_bend_points_scale_dynamic_rpn_ranges() {
    let events = vec![
        PitchBendEvent {
            tick: 0,
            value: 8192,
        },
        PitchBendEvent {
            tick: 240,
            value: 16383,
        },
        PitchBendEvent {
            tick: 480,
            value: 16383,
        },
        PitchBendEvent {
            tick: 720,
            value: 0,
        },
    ];
    let ranges = vec![
        PitchBendRangeEvent {
            tick: 0,
            semitones: 12.0,
        },
        PitchBendRangeEvent {
            tick: 480,
            semitones: 24.0,
        },
    ];
    let points = pitch_bend_points(&events, &ranges, 480, 2.0, 24.0).unwrap();
    let at = |beat: f64| {
        points
            .iter()
            .rev()
            .find(|(time, _)| *time <= beat)
            .map(|(_, value)| *value)
            .unwrap()
    };
    assert_eq!(at(0.0), 0.0);
    assert!((at(0.5) - 4095.5).abs() < 1e-9);
    assert!((at(1.0) - 8191.0).abs() < 1e-9);
    assert!((at(1.5) + 8192.0).abs() < 1e-9);
}

#[test]
fn embedded_default_template_passes_live_id_validation() {
    let xml = read_als_bytes(crate::DEFAULT_TEMPLATE_BYTES, crate::DEFAULT_TEMPLATE_LABEL).unwrap();
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
            pitch_bend_ranges: &no_pb_ranges(),
            pitch_bend_range: 2.0,
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
            pitch_bend_ranges: &no_pb_ranges(),
            pitch_bend_range: 2.0,
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
