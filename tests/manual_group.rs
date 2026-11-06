use std::path::PathBuf;

use blacklive::{ConvertOptions, convert_midi_to_als, inspect_template};

#[test]
#[ignore = "manual integration test; set BLACKLIVE_TEMPLATE, BLACKLIVE_MIDI, BLACKLIVE_OUT"]
fn convert_real_group_template() {
    let template = PathBuf::from(std::env::var("BLACKLIVE_TEMPLATE").expect("template path"));
    let midi = PathBuf::from(std::env::var("BLACKLIVE_MIDI").expect("midi path"));
    let output = PathBuf::from(std::env::var("BLACKLIVE_OUT").expect("output path"));

    let report = inspect_template(&template).unwrap();
    println!("template clips:");
    for clip in &report.clips {
        println!(
            "#{} group={:?} {} / {} notes={}",
            clip.candidate_index, clip.group_name, clip.location, clip.track_name, clip.note_count
        );
    }
    let clip = report
        .clips
        .iter()
        .find(|clip| clip.group_id.is_some())
        .expect("a clip inside a group");
    println!(
        "selected #{} {}",
        clip.candidate_index,
        clip.display_label()
    );

    let result = convert_midi_to_als(
        &midi,
        &template,
        &output,
        &ConvertOptions {
            template_clip_index: Some(clip.candidate_index),
            ..ConvertOptions::default()
        },
    )
    .unwrap();
    println!("output {}", result.output_path.display());
    println!(
        "groups={} tracks={} notes={}",
        result.groups.len(),
        result.groups.iter().map(|g| g.tracks.len()).sum::<usize>(),
        result.used_note_count
    );
    for group in &result.groups {
        println!("group {} {:?}", group.name, group.channel);
        for track in &group.tracks {
            println!(
                "  {} source={} notes={} length={}",
                track.name, track.source_track, track.note_count, track.clip_length_beats
            );
        }
    }
    for warning in &result.warnings {
        println!("warning: {warning}");
    }

    let out_report = inspect_template(&result.output_path).unwrap();
    println!("output clips:");
    for clip in &out_report.clips {
        println!(
            "#{} group={:?} {} / {} notes={}",
            clip.candidate_index, clip.group_name, clip.location, clip.track_name, clip.note_count
        );
    }
    assert_eq!(
        out_report
            .clips
            .iter()
            .filter(|clip| clip.group_id.is_some())
            .count(),
        1
    );
    let group_clip = out_report
        .clips
        .iter()
        .find(|clip| clip.group_id.is_some())
        .unwrap();
    assert_eq!(group_clip.note_count, result.used_note_count);
}
