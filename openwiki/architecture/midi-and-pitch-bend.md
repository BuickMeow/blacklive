---
type: architecture
title: MIDI 解析与 Pitch Bend / RPN
description: Blacklive 如何解析 MIDI 音符、Tempo、CC、Pitch Bend 与 RPN 0,0，并把动态弯音范围折算进 Ableton 的 PB 包络。
tags: [midi, pitch-bend, rpn, tempo, parser, ableton]
verified:
  - by: openwiki/0.9.0
    at: 2026-11-07T02:37:32.818Z
sources:
  - id: openwiki-source-379215f33156a03d017739bd
    resource: repo://src/als/clip.rs
  - id: openwiki-source-89b2fadf9596bee25628864e
    resource: repo://src/als/devices.rs
  - id: openwiki-source-71553ff6b7004ea001bf94c7
    resource: repo://src/convert/options.rs
  - id: openwiki-source-b912ad5e2f8c8147e20438c3
    resource: repo://src/convert/pipeline.rs
  - id: openwiki-source-1501e6c3724d129df40c7abf
    resource: repo://src/midi/parse.rs
generated: { by: "opencode", at: "2026-11-07T02:37:32.818Z" }
---

# MIDI 解析与 Pitch Bend / RPN

`src/midi/parse.rs` 负责把标准 MIDI 文件变成：

- `Vec<Note>`：绝对 beat 坐标的音符
- `MidiStats`：通道、Tempo、拍号、CC、Pitch Bend、RPN、复音等统计

之后 `src/convert/pipeline.rs` 决定固定 Pitch Bend 范围，`src/als/clip.rs` 把 PB 值写成 ClipEnvelope，`src/als/devices.rs` 把范围写进被克隆的乐器。

## MIDI 解析入口

`parse_midi_file` 读文件后调用 `parse_midi_bytes`。解析器只接受：

- PPQ/Metrical 时间基准
- Format 0（单轨）或 Format 1（并行轨）

以下情况直接报错：

- SMPTE 时间码
- MIDI Format 2（Sequential）
- PPQ 为 0

## 音符配对

音符不是按顺序简单配对，而是用 **LIFO**（后进先出）策略：

- 以 `(track, channel, key)` 为栈
- 后到的 Note On 压在先到的上面
- Note Off 释放该栈最近的一个 Note On

这样更接近大多数合成器的 per-key note stack 行为，也能处理同音高重叠。

几个边界：

- Note On 和 Note Off 在同一 tick：时长按 1 tick 写入，并计入 `zero_duration_notes`
- 找不到 Note On 的 Note Off：忽略并计数
- 没有 Note Off 的 Note On：直接忽略，避免生成超长挂音
- 一个音符都没有：解析失败

## Tempo 与拍号

- Tempo 事件按 `(tick, order)` 排序；同一 tick 后者覆盖前者
- 如果第一个 Tempo 事件不在 tick 0，会补一个 120 BPM 的默认值
- 多个 Tempo 点会产生 warning，并交给 `src/als/tempo.rs` 写 Master Tempo 自动化
- 拍号只保留第一个写进 Clip 显示；多拍号会 warning，播放不受影响

## RPN 0,0：Pitch Bend Sensitivity

MIDI 的 RPN 0,0 用四个 CC 组合表达：

| CC | 含义 |
|---|---|
| 101 | RPN MSB |
| 100 | RPN LSB |
| 6 | Data Entry MSB（半音） |
| 38 | Data Entry LSB（音分） |

解析器为每个 channel 维护一组 RPN 状态。只有当前 RPN 为 `0,0` 时，CC6/CC38 才会生成 `PitchBendRangeEvent`：

```text
range_semitones = data_msb + data_lsb / 100
```

没有 RPN 数据时，按 MIDI 标准默认 **±2 半音** 处理。

## 为什么要“折算”而不是直接写 RPN

Ableton Live 的 Simpler 并不把 MIDI Clip 里的 RPN 当成设备参数。Simpler 的弯音范围是一个静态设备参数：

```xml
<OriginalSimpler>
  <Globals>
    <PitchBendRange Value="5" />
  </Globals>
</OriginalSimpler>
```

因此直接写 CC101/100/6/38 包络不会改变 Simpler 的弯音范围。Blacklive 的做法是：

1. 扫描 MIDI 中所有 RPN 0,0，取最大范围（最低 2，最高 48 半音）
2. 把这个固定范围写进克隆乐器的 `PitchBendRange` / `PitchbendRange`
3. 按 RPN 时间线把每个 PB 事件折算成固定范围下的值

折算公式在 `pitch_bend_points`：

```text
signed = raw_pb - 8192
scaled = signed × current_rpn_range / fixed_range
scaled = clamp(scaled, -8192, 8191)
```

这样 Live 里看到的是一个固定范围的 PB 包络，但听感上仍然保留 2、6、12、24 半音的分段变化。

## 固定范围的选择

`ConvertOptions.pitch_bend_range` 为 `None` 时自动：

- 取 MIDI 中出现的最大 RPN 范围
- 至少 2 半音
- 最多 48 半音（Simpler 上限）

超过 48 会 warning 并夹到 48。也可以手动指定固定范围。

## 设备参数写入

`set_pitch_bend_range` 会遍历克隆乐器，匹配大小写不敏感的 `PitchBendRange` / `PitchbendRange`，写入固定值。这样 Simpler、Sampler、Operator、Drift、Meld 等 Live 内置乐器可以共用同一套 PB 折算。

第三方 VST 通常不会在 `.als` XML 里暴露这个参数；这种情况下需要在插件里手动设成相同范围，否则听感会按插件自己的范围走。

## 当前边界

- 只解析 RPN 0,0；其它 RPN 不处理
- Data Entry LSB 按 1/100 半音解释
- 不写 Channel Pressure / Poly Pressure
- Pitch Bend 默认写入，CC 默认只统计不写入
- 动态 RPN 已折算进 PB 包络，但仍需要在 Live 里试听确认

## 相关阅读

- [Clip、音符与自动化](clip-and-automation.md)
- [关键设计与取舍](../decisions.md)
- [当前限制与下一步](../limitations.md)
