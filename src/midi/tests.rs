use super::parse::max_polyphony_ticks;
use super::*;

#[test]
fn max_polyphony_counts_off_before_on_at_same_tick() {
    let notes = vec![
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
            key: 60,
            start: 1.0,
            duration: 1.0,
            velocity: 100.0,
            off_velocity: 64.0,
            channel: 0,
            source_track: 0,
        },
    ];
    assert_eq!(max_polyphony_ticks(&notes, 480), 1);
}

#[test]
fn parses_rpn_pitch_bend_range_changes() {
    let mut track = Vec::new();
    fn cc(track: &mut Vec<u8>, controller: u8, value: u8) {
        track.extend_from_slice(&[0x00, 0xB0, controller, value]);
    }
    track.extend_from_slice(&[0x00, 0x90, 0x3C, 0x64]);
    cc(&mut track, 101, 0);
    cc(&mut track, 100, 0);
    cc(&mut track, 6, 12);
    track.extend_from_slice(&[0x83, 0x60, 0xE0, 0x7F, 0x7F]);
    cc(&mut track, 101, 0);
    cc(&mut track, 100, 0);
    cc(&mut track, 6, 24);
    track.extend_from_slice(&[0x83, 0x60, 0xE0, 0x7F, 0x7F]);
    track.extend_from_slice(&[0x83, 0x60, 0x80, 0x3C, 0x40]);
    track.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);

    let mut smf = Vec::new();
    smf.extend_from_slice(b"MThd");
    smf.extend_from_slice(&6u32.to_be_bytes());
    smf.extend_from_slice(&0u16.to_be_bytes());
    smf.extend_from_slice(&1u16.to_be_bytes());
    smf.extend_from_slice(&480u16.to_be_bytes());
    smf.extend_from_slice(b"MTrk");
    smf.extend_from_slice(&(track.len() as u32).to_be_bytes());
    smf.extend_from_slice(&track);

    let parsed = parse_midi_bytes(&smf).unwrap();
    let ranges = parsed.stats.pitch_bend_ranges.get(&0).unwrap();
    assert_eq!(ranges.len(), 2);
    assert_eq!(ranges[0].tick, 0);
    assert_eq!(ranges[0].semitones, 12.0);
    assert_eq!(ranges[1].tick, 480);
    assert_eq!(ranges[1].semitones, 24.0);
    assert_eq!(parsed.stats.rpn_pitch_bend_range_count, 2);
    assert_eq!(parsed.stats.pitch_bends.get(&0).unwrap().len(), 2);
}
