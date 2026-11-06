use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use blacklive::{
    ConvertOptions, convert_midi_to_als_with_default_template, inspect_default_template,
    inspect_template,
};

#[test]
fn embedded_template_is_a_clean_a01_routing_group() {
    let report = inspect_default_template().unwrap();
    assert_eq!(report.clips.len(), 1);

    let clip = &report.clips[0];
    assert_eq!(clip.group_name.as_deref(), Some("A01"));
    assert_eq!(clip.location, blacklive::ClipLocation::Arrangement);
    assert_eq!(clip.note_count, 0);

    let group = report
        .tracks
        .iter()
        .find(|track| track.kind == blacklive::TrackKind::Group)
        .expect("A01 GroupTrack");
    assert_eq!(group.name, "A01");

    let instrument = report
        .tracks
        .iter()
        .find(|track| track.group_name.as_deref() == Some("A01") && track.has_instrument)
        .expect("A01 instrument track");
    let routed = report
        .tracks
        .iter()
        .find(|track| track.group_name.as_deref() == Some("A01") && !track.has_instrument)
        .expect("A01 MIDI track");
    assert_eq!(routed.midi_target, instrument.id);
    assert!(
        report.warnings.is_empty(),
        "unexpected template warnings: {:?}",
        report.warnings
    );
}

#[test]
fn converts_pure_piano_midi_with_the_embedded_template() {
    let midi = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pure_piano.mid");
    let output = unique_output();

    let result =
        convert_midi_to_als_with_default_template(&midi, &output, &ConvertOptions::default())
            .unwrap();

    assert_eq!(result.template_label, blacklive::DEFAULT_TEMPLATE_LABEL);
    assert_eq!(result.used_note_count, 7);
    assert_eq!(result.groups.len(), 2);

    let a01 = result
        .groups
        .iter()
        .find(|group| group.name == "A01")
        .expect("A01 result");
    assert_eq!(a01.tracks.len(), 3);
    assert_eq!(
        a01.tracks
            .iter()
            .filter(|track| track.note_count > 0)
            .count(),
        2
    );
    assert_eq!(
        a01.tracks
            .iter()
            .filter(|track| track.note_count == 0)
            .count(),
        1
    );

    let a02 = result
        .groups
        .iter()
        .find(|group| group.name == "A02")
        .expect("A02 result");
    assert_eq!(a02.tracks.len(), 1);
    assert_eq!(a02.tracks[0].note_count, 2);

    let output_report = inspect_template(&output).unwrap();
    assert!(
        output_report
            .tracks
            .iter()
            .any(|track| track.name == "Silent ch1")
    );
    assert!(
        !output_report
            .tracks
            .iter()
            .any(|track| track.name == "Meta only")
    );
    let output_notes: usize = output_report.clips.iter().map(|clip| clip.note_count).sum();
    assert_eq!(output_notes, result.used_note_count);

    let _ = std::fs::remove_file(&output);
}

fn unique_output() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "blacklive-pure-piano-{}-{nanos}.als",
        std::process::id()
    ))
}

#[test]
fn pitch_bend_is_written_to_the_longest_source_track() {
    let midi = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pb_longest_track.mid");
    let output = unique_output();

    let result =
        convert_midi_to_als_with_default_template(&midi, &output, &ConvertOptions::default())
            .unwrap();
    let a01 = result
        .groups
        .iter()
        .find(|group| group.name == "A01")
        .expect("A01 result");
    assert_eq!(a01.tracks.len(), 3);
    assert_eq!(a01.tracks[0].name, "Short");
    assert_eq!(a01.tracks[0].clip_length_beats, 1.0);
    assert_eq!(a01.tracks[1].name, "Long");
    assert_eq!(a01.tracks[1].clip_length_beats, 4.0);
    assert_eq!(a01.tracks[2].name, "PB only");
    assert_eq!(a01.tracks[2].note_count, 0);

    let xml = blacklive::als::read_als_xml(&output).unwrap();
    let root = xmltree::Element::parse(std::io::Cursor::new(xml)).unwrap();
    let mut clips = Vec::new();
    collect_elements(&root, "MidiClip", &mut clips);

    let short = clips
        .iter()
        .find(|clip| clip_name(clip) == Some("pb_longest_track A01-01"))
        .expect("short clip");
    let long = clips
        .iter()
        .find(|clip| clip_name(clip) == Some("pb_longest_track A01-02"))
        .expect("long clip");

    assert_eq!(count_elements(short, "ClipEnvelope"), 0);
    assert_eq!(count_elements(long, "ClipEnvelope"), 1);

    let mut events = Vec::new();
    collect_elements(long, "FloatEvent", &mut events);
    let points: Vec<(Option<&String>, Option<&String>)> = events
        .iter()
        .map(|event| (event.attributes.get("Time"), event.attributes.get("Value")))
        .collect();
    assert!(points.iter().any(|(time, value)| {
        time.map(String::as_str) == Some("3") && value.map(String::as_str) == Some("8191")
    }));
    assert!(points.iter().any(|(time, value)| {
        time.map(String::as_str) == Some("4") && value.map(String::as_str) == Some("0")
    }));

    let _ = std::fs::remove_file(&output);
}

fn clip_name(clip: &xmltree::Element) -> Option<&str> {
    clip.get_child("Name")?
        .attributes
        .get("Value")
        .map(String::as_str)
}

fn collect_elements<'a>(
    element: &'a xmltree::Element,
    name: &str,
    out: &mut Vec<&'a xmltree::Element>,
) {
    if element.name == name {
        out.push(element);
    }
    for child in element.children.iter().filter_map(|node| node.as_element()) {
        collect_elements(child, name, out);
    }
}

fn count_elements(element: &xmltree::Element, name: &str) -> usize {
    let mut elements = Vec::new();
    collect_elements(element, name, &mut elements);
    elements.len()
}
