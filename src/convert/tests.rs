use super::filter::trim_same_pitch_overlaps;
use super::*;
use crate::midi::Note;
use std::collections::BTreeSet;

#[test]
fn channel_filter_accepts_ranges() {
    let parsed = parse_channel_filter("0,2-4, 7").unwrap().unwrap();
    assert_eq!(parsed, BTreeSet::from([0, 2, 3, 4, 7]));
    assert!(parse_channel_filter("").unwrap().is_none());
    assert!(parse_channel_filter("all").unwrap().is_none());
    assert!(parse_channel_filter("16").is_err());
}

#[test]
fn trim_overlaps_shortens_previous_note() {
    let mut notes = vec![
        Note {
            key: 60,
            start: 0.0,
            duration: 2.0,
            velocity: 100.0,
            off_velocity: 64.0,
            channel: 0,
            source_track: 0,
        },
        Note {
            key: 60,
            start: 1.0,
            duration: 1.0,
            velocity: 90.0,
            off_velocity: 64.0,
            channel: 0,
            source_track: 0,
        },
    ];
    assert_eq!(trim_same_pitch_overlaps(&mut notes), 1);
    assert_eq!(notes[0].duration, 1.0);
}
