---
type: workflow
title: 转换流程与选项
description: 面向使用者的转换行为说明：通道、移调、BPM、源轨拆分、空轨、Pitch Bend、CC 和效果器选项。
tags: [workflow, conversion, options, midi, ableton]
verified:
  - by: openwiki/0.9.0
    at: 2026-11-07T02:37:32.818Z
sources:
  - id: openwiki-source-89b2fadf9596bee25628864e
    resource: repo://src/als/devices.rs
  - id: openwiki-source-96afe7c1afce95b4d368669d
    resource: repo://src/als/inject/clone.rs
  - id: openwiki-source-92c1f61411bf3152141942da
    resource: repo://src/convert/filter.rs
  - id: openwiki-source-3f15ae5f13c9890b73329643
    resource: repo://src/convert/grouping.rs
  - id: openwiki-source-71553ff6b7004ea001bf94c7
    resource: repo://src/convert/options.rs
  - id: openwiki-source-b912ad5e2f8c8147e20438c3
    resource: repo://src/convert/pipeline.rs
generated: { by: "opencode", at: "2026-11-07T02:37:32.818Z" }
---

# 转换流程与选项

这一页解释一次转换会怎样处理 MIDI，以及每个选项的实际影响。入口是 `ConvertOptions`，默认值在 `src/convert/options.rs`。

## 默认行为

默认 `ConvertOptions` 的关键值：

| 选项 | 默认 | 影响 |
|---|---|---|
| `split_source_tracks` | `true` | 每个源 MIDI 轨生成一条子轨 |
| `keep_empty_tracks` | `true` | 有 MIDI 事件但无音符的源轨保留为空轨 |
| `write_pitch_bend` | `true` | 写 Pitch Bend ClipEnvelope |
| `pitch_bend_range` | `None` | 自动取 MIDI 中最大 RPN 范围（最低 2，最高 48） |
| `write_cc_automation` | `false` | CC 只统计，不写自动化 |
| `strip_instrument_effects` | `true` | 关闭克隆乐器内部 Reverb/Delay/Chorus/Echo |
| `include_drum_channel` | `false` | 默认排除 GM 鼓通道（0-based index 9） |
| `transpose` | `0` | 不移调 |
| `min_note_beats` | `0.0` | 保持 MIDI 原时长 |
| `tail_beats` | `0.0` | 不额外加尾部留白 |
| `start_beats` | `0.0` | 从 Arrangement 第 1 小节开始 |
| `trim_same_pitch` | `false` | 同音高重叠不截短 |

## 通道过滤

- `channels = None`：所有通道，但默认排除 GM 鼓通道
- `include_drum_channel = true`：把鼓通道也当普通通道处理
- `channels = Some(...)`：只保留指定通道

被过滤掉的音符计入 `dropped_by_channel`，最终在 `ConversionResult` 里报告。

## 移调与音符边界

- `transpose` 以半音为单位
- 移调后超出 MIDI 0–127 的音符直接丢弃
- 丢弃数量计入 `dropped_by_transpose`
- `min_note_beats > 0` 时，短于该值的音符会被拉长
- `tail_beats` 会加到每条子轨 Clip 的末尾
- `trim_same_pitch = true` 时，同音高前音会在下一个同音高音符开始时被截短

## BPM

BPM 优先级：

```text
bpm_override
  > MIDI 初始 BPM
  > 模板 BPM
  > 120
```

- `bpm_override` 有值时，MIDI 的变速点不会写入 Tempo 自动化
- 没有 MIDI Tempo 时，使用模板 BPM
- BPM 必须是正数

## 源轨拆分与空轨

`split_source_tracks = true`：

- 同一通道的每个源 MIDI 轨变成一条子轨
- 子轨名优先使用 MIDI Track Name
- 空名回退到 `A01-01` 这样的名字

`split_source_tracks = false`：

- 同一通道的所有源轨合并成一条子轨
- 所有音符的 `source_track` 视为 0

`keep_empty_tracks = true` 且拆分开启时：

- 源轨在该通道有 MIDI 事件但没有音符 → 保留为空 MIDI 轨
- 空轨没有 Clip、没有额外乐器
- 完全没有 MIDI 事件的 conductor/meta-only 轨不会创建

## Pitch Bend

- 默认写入
- 目标是 `ControllerTargets.0`
- 值域是 Live 的 -8192..8191，中心 0
- 按 RPN 0,0 时间线折算到固定范围
- 只写在该通道最长有音符子轨的 Clip 上

## CC

- 默认只解析、统计，不写入
- GUI 会显示每通道 CC 数量
- 转换时会给 warning
- 以后需要 CC→效果映射表再打开

## 效果器

- `strip_instrument_effects = true`：关闭克隆乐器内部的 Reverb/Delay/Chorus/Echo
- Group/Return 效果保留
- 默认是干声；需要空间感时在 Group 或 Return 上手动加
- 不会自动添加 EQ/Compressor/Reverb

## 模板 Clip 选择

- `template_clip_index`：按索引选
- `template_clip_name`：按名称选
- 都不给时：优先 Group 内 Arrangement Clip，再退到其它 Clip

## 相关阅读

- [项目总览](../overview.md)
- [GUI 使用与代码路径](gui.md)
- [当前限制与下一步](../limitations.md)
