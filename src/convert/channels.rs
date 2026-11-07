use super::*;
use std::collections::BTreeSet;

/// Parse "0,1,2", "0-3", or "" / "all" into a channel set.
pub fn parse_channel_filter(input: &str) -> Result<Option<BTreeSet<u8>>> {
    let input = input.trim();
    if input.is_empty() || input.eq_ignore_ascii_case("all") {
        return Ok(None);
    }

    let mut channels = BTreeSet::new();
    for part in input.split([',', '，']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((a, b)) = part.split_once('-') {
            let a = parse_channel(a)?;
            let b = parse_channel(b)?;
            if a > b {
                bail!("通道范围 {a}-{b} 顺序反了");
            }
            for ch in a..=b {
                channels.insert(ch);
            }
        } else {
            channels.insert(parse_channel(part)?);
        }
    }
    if channels.is_empty() {
        Ok(None)
    } else {
        Ok(Some(channels))
    }
}

fn parse_channel(s: &str) -> Result<u8> {
    let ch: u8 = s
        .trim()
        .parse()
        .with_context(|| format!("无效的 MIDI 通道「{s}」"))?;
    if ch > 15 {
        bail!("MIDI 通道必须在 0–15 之间，收到 {ch}");
    }
    Ok(ch)
}
