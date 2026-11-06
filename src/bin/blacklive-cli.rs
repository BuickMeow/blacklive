use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use blacklive::{ConvertOptions, convert_midi_to_als, parse_channel_filter, parse_midi_file};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "blacklive-cli",
    version,
    about = "把 MIDI 注入 Ableton Live 模板，生成可直接打开的 .als"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// 解压/分析模板 .als，列出轨道、Clip、占位音符和 Live 版本
    Inspect {
        template: PathBuf,
        /// 同时把解压后的 XML 写到这个路径，便于用编辑器检查标签
        #[arg(long)]
        xml_out: Option<PathBuf>,
    },
    /// 只解析 MIDI，显示通道、变速、踏板、最大复音等诊断信息
    Midi { midi: PathBuf },
    /// 转换 MIDI -> .als
    Convert {
        midi: PathBuf,
        template: PathBuf,
        /// 省略时输出到 MIDI 同目录的同名 .als
        output: Option<PathBuf>,
        /// 按模板中 Clip 的序号选择（见 inspect 输出）
        #[arg(long)]
        clip_index: Option<usize>,
        /// 按模板中 Clip 的名称选择；不填则优先 Arrangement 的第一个 Clip
        #[arg(long = "template-clip")]
        template_clip: Option<String>,
        /// 只保留这些 MIDI 通道，例如 0,1,2 或 0-3；留空=全部（默认排除 10）
        #[arg(long)]
        channels: Option<String>,
        /// 显式包含 GM 鼓通道（0-based index 9）
        #[arg(long)]
        include_drum_channel: bool,
        /// 移调半音数
        #[arg(long, default_value_t = 0)]
        transpose: i32,
        /// 覆盖 BPM；留空时使用 MIDI 初始 BPM，再退回模板 BPM
        #[arg(long)]
        bpm: Option<f64>,
        /// 最短音符时值（beats），0 = 保留 MIDI 原时值
        #[arg(long, default_value_t = 0.0)]
        min_note: f64,
        /// Clip 末尾额外留白（beats）
        #[arg(long, default_value_t = 0.0)]
        tail: f64,
        /// Arrangement 起点（beats），0 = 第 1 小节
        #[arg(long, default_value_t = 0.0)]
        start: f64,
        /// 同音高重叠时截短前一个音
        #[arg(long)]
        trim_same_pitch: bool,
        /// 写入完成后用系统默认程序打开 .als
        #[arg(long)]
        open: bool,
        /// 输出 Clip 名称；留空=MIDI 文件名
        #[arg(long)]
        clip_name: Option<String>,
        /// 输出轨道名称；留空=Clip 名称
        #[arg(long)]
        track_name: Option<String>,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("错误: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Inspect { template, xml_out } => inspect(&template, xml_out.as_deref()),
        Commands::Midi { midi } => inspect_midi(&midi),
        Commands::Convert {
            midi,
            template,
            output,
            clip_index,
            template_clip,
            channels,
            include_drum_channel,
            transpose,
            bpm,
            min_note,
            tail,
            start,
            trim_same_pitch,
            open,
            clip_name,
            track_name,
        } => {
            let output = output.unwrap_or_else(|| midi.with_extension("als"));
            let options = ConvertOptions {
                template_clip_index: clip_index,
                template_clip_name: template_clip,
                channels: parse_channel_filter(channels.as_deref().unwrap_or(""))?,
                include_drum_channel,
                transpose,
                bpm_override: bpm,
                min_note_beats: min_note,
                tail_beats: tail,
                start_beats: start,
                clip_name,
                track_name,
                trim_same_pitch,
            };
            let result = convert_midi_to_als(&midi, &template, &output, &options)?;
            print_result(&result);
            if open {
                open_path(&result.output_path)?;
            }
            Ok(())
        }
    }
}

fn inspect(template: &Path, xml_out: Option<&Path>) -> Result<()> {
    if let Some(xml_out) = xml_out {
        blacklive::als::dump_template_xml(template, xml_out)?;
        println!("已写出解压 XML: {}", xml_out.display());
        println!();
    }
    let report = blacklive::inspect_template(template)?;
    println!("模板: {}", report.path.display());
    println!(
        "Live:  MajorVersion={} MinorVersion={} Creator={} Revision={}",
        report.major_version, report.minor_version, report.creator, report.revision
    );
    println!(
        "BPM:   {}",
        report
            .bpm
            .map(blacklive::format_num)
            .unwrap_or_else(|| "未找到".to_string())
    );
    println!();
    println!("MIDI 轨道 ({}):", report.tracks.len());
    for track in &report.tracks {
        println!(
            "  #{} {}{}",
            track.index,
            track.name,
            if track.has_instrument {
                ""
            } else {
                "  [!] 未检测到乐器"
            }
        );
        if !track.devices.is_empty() {
            println!("      设备: {}", track.devices.join(", "));
        }
    }
    println!();
    println!("Clip ({}):", report.clips.len());
    for clip in &report.clips {
        let mark = if Some(clip.candidate_index) == report.recommended_clip {
            "*"
        } else {
            " "
        };
        println!(
            "{} #{} {}",
            mark,
            clip.candidate_index,
            clip.display_label()
        );
        println!(
            "      Time={} Current={}..{} Loop={}..{} LoopOn={}",
            blacklive::format_num(clip.time),
            blacklive::format_num(clip.current_start),
            blacklive::format_num(clip.current_end),
            blacklive::format_num(clip.loop_start),
            blacklive::format_num(clip.loop_end),
            clip.loop_on
        );
    }
    if let Some(index) = report.recommended_clip {
        println!();
        println!("默认会替换: #{index}（可用 --clip-index 或 --template-clip 指定）");
    }
    if !report.warnings.is_empty() {
        println!();
        println!("警告:");
        for warning in &report.warnings {
            println!("  - {warning}");
        }
    }
    println!();
    println!("检查 XML 中的音符节点:");
    println!(
        "  blacklive-cli inspect {} --xml-out /tmp/template.xml",
        template.display()
    );
    println!("  grep -n '<MidiClip' /tmp/template.xml");
    println!("  grep -n '<MidiNoteEvent' /tmp/template.xml | head");
    Ok(())
}

fn inspect_midi(midi: &Path) -> Result<()> {
    let parsed = parse_midi_file(midi)?;
    let stats = &parsed.stats;
    println!("MIDI: {}", midi.display());
    println!(
        "格式: {}  PPQ: {}  轨道数: {}",
        stats.format, stats.ppq, stats.track_count
    );
    if !stats.track_names.is_empty() {
        println!("轨道名: {}", stats.track_names.join(" | "));
    }
    println!(
        "音符: {}  Pitch: {}..{}  时长: {} beats",
        stats.note_count,
        stats.min_pitch,
        stats.max_pitch,
        blacklive::format_num(stats.duration_beats)
    );
    println!(
        "时值: min={} avg={} max={} beats",
        blacklive::format_num(stats.min_duration_beats),
        blacklive::format_num(stats.avg_duration_beats),
        blacklive::format_num(stats.max_duration_beats)
    );
    println!("最大复音: {}", stats.max_polyphony);
    println!(
        "初始 BPM: {}  Tempo 事件: {}  有效变速点: {}",
        stats
            .initial_bpm
            .map(blacklive::format_num)
            .unwrap_or_else(|| "无".to_string()),
        stats.tempo_event_count,
        stats.tempo_map.len()
    );
    if stats.tempo_map.len() > 1 {
        let points = stats
            .tempo_map
            .iter()
            .take(8)
            .map(|t| {
                format!(
                    "{}→{}",
                    blacklive::format_num(t.tick as f64 / stats.ppq as f64),
                    blacklive::format_num(t.bpm)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "变速点 (beat→BPM): {points}{}",
            if stats.tempo_map.len() > 8 {
                " …"
            } else {
                ""
            }
        );
    }
    if !stats.time_signatures.is_empty() {
        println!(
            "拍号: {}",
            stats
                .time_signatures
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!("通道音符数:");
    for (channel, count) in &stats.notes_per_channel {
        let programs = stats
            .programs
            .get(channel)
            .map(|p| p.iter().map(u8::to_string).collect::<Vec<_>>().join(","))
            .unwrap_or_default();
        println!(
            "  ch{:>2}: {}{}",
            channel,
            count,
            if programs.is_empty() {
                String::new()
            } else {
                format!("  programs=[{programs}]")
            }
        );
    }
    println!(
        "CC64={}  PitchBend={}  同 key 重叠对={}  未配对 NoteOff={}  未结束 NoteOn={}",
        stats.sustain_event_count,
        stats.pitch_bend_event_count,
        stats.overlapping_same_pitch_pairs,
        stats.unmatched_note_offs,
        stats.unterminated_note_ons
    );
    if !stats.warnings.is_empty() {
        println!();
        println!("警告:");
        for warning in &stats.warnings {
            println!("  - {warning}");
        }
    }
    Ok(())
}

fn print_result(result: &blacklive::ConversionResult) {
    println!("输出: {}", result.output_path.display());
    println!(
        "模板 Clip: {} ({})",
        result.template_clip,
        if result.arrangement_created {
            "已克隆到 Arrangement"
        } else {
            "Arrangement"
        }
    );
    println!(
        "音符: 源 {} -> 写入 {}  (通道丢弃 {}, 移调丢弃 {}, 拉短 {}, 重叠截短 {})",
        result.source_note_count,
        result.used_note_count,
        result.dropped_by_channel,
        result.dropped_by_transpose,
        result.clamped_short_notes,
        result.trimmed_overlaps
    );
    println!(
        "BPM: {}  Clip: {}..{} beats  名称: {} / 轨道: {}{}",
        blacklive::format_num(result.bpm),
        blacklive::format_num(result.clip_start_beats),
        blacklive::format_num(result.clip_start_beats + result.clip_length_beats),
        result.clip_name,
        result.track_name,
        if result.tempo_automation {
            "  [含 Tempo 自动化]"
        } else {
            ""
        }
    );
    println!(
        "最大复音: {}  通道: {}",
        result.midi_stats.max_polyphony,
        result
            .midi_stats
            .notes_per_channel
            .keys()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    if !result.warnings.is_empty() {
        println!();
        println!("警告:");
        for warning in &result.warnings {
            println!("  - {warning}");
        }
    }
    println!();
    println!("下一步: 用目标 Ableton Live 版本打开这个 .als，先试听前几小节。");
}

fn open_path(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = Command::new("xdg-open");

    command
        .arg(path)
        .spawn()
        .with_context(|| format!("无法打开 {}", path.display()))?;
    Ok(())
}
