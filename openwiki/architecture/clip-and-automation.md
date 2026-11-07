---
type: architecture
title: Clip、音符与自动化
description: Blacklive 如何把 MIDI 音符、Pitch Bend、CC 与 Tempo 写入 Ableton 的 Arrangement Clip 和 Master 自动化。
tags: [ableton, als, clip, midi, automation, tempo, pitch-bend]
verified:
  - by: openwiki/0.9.0
    at: 2026-11-07T02:37:32.818Z
sources:
  - id: openwiki-source-379215f33156a03d017739bd
    resource: repo://src/als/clip.rs
  - id: openwiki-source-96afe7c1afce95b4d368669d
    resource: repo://src/als/inject/clone.rs
  - id: openwiki-source-085514bf62cef05fc56071f9
    resource: repo://src/als/tempo.rs
generated: { by: "opencode", at: "2026-11-07T02:37:32.818Z" }
---

# Clip、音符与自动化

这一层负责把已经解析、过滤、分组后的 MIDI 数据写成 Ableton Live 能读取的 XML 结构。入口在 `src/als/clip.rs`，Tempo 部分在 `src/als/tempo.rs`，实际把 Clip 挂到轨道上的逻辑在 `src/als/inject/clone.rs`。

## 数据流

<!-- openwiki: mermaid parse failed and this diagram was converted to a text fence so it does not break rendering. Fix the diagram source and restore the mermaid fence. Parser error: Heuristic: an unescaped angle bracket inside a label breaks rendering; rephrase the label. -->
```text
flowchart LR
    A[ChildPlan<br/>notes + clip_length] --> B[clone 子轨]
    B --> C[write_child_clip]
    C --> D[edit_clip]
    D --> E[replace_notes]
    D --> F[Loop / Scroller / TimeSignature]
    C --> G[clear_clip_envelopes]
    G --> H[add_midi_automation]
    H --> I[Pitch Bend / CC ClipEnvelope]
    J[Tempo Map] --> K[set_tempo]
    K --> L[MainTrack AutomationEnvelope]
```

## Clip 的时序字段

`edit_clip` 把模板里的占位 Clip 改造成目标 Clip：

- `Time` 和 `CurrentStart` 设为 Arrangement 起点
- `CurrentEnd` 设为 `start + length`
- `LoopStart = 0`、`LoopEnd = length`
- `LoopOn = false`，避免 Live 自动循环
- `OutMarker`、`HiddenLoopStart`、`HiddenLoopEnd` 同步到 Clip 长度
- `ScrollerTimePreserver.LeftTime = 0`、`RightTime = length`
- `TimeSelection` 归零，`FreezeStart` / `FreezeEnd` 归零
- 如果 MIDI 有拍号，会更新模板 `TimeSignature` 里的 `RemoteableTimeSignature`

这些字段的写入集中在 `src/als/clip.rs` 的 `edit_clip`。

## 音符写入

音符不是逐个平铺，而是按音高分组：

1. 每个 MIDI pitch 对应一个 `KeyTrack`
2. `KeyTrack.Id` 从 0 开始按排序后的音高递增
3. 每个 `KeyTrack` 下的 `MidiNoteEvent` 写 `Time`、`Duration`、`Velocity`、`OffVelocity`
4. 模板 Clip 如果带 `NoteIdGenerator`，才写 `NoteId`
5. `NoteIdGenerator.NextId` 更新为 `notes.len() + 1`

写入前会清空模板里的 `PerNoteEventStore.EventLists` 和 `NoteProbabilityGroups`，避免旧 Clip 的残留概率/事件数据混进输出。

## ClipEnvelope 与自动化

写入新 Clip 前会调用 `clear_clip_envelopes` 清空模板 Clip 的 `Envelopes/Envelopes`，然后按需追加：

- **Pitch Bend**：目标是 `ControllerTargets.0`
- **CC n**：目标是 `ControllerTargets.(n+1)`，例如 CC64 对应 `.65`
- CC 写入目前默认关闭；Pitch Bend 默认开启
- 每个 `ClipEnvelope` 包含 `EnvelopeTarget/PointeeId`、`Automation/Events/FloatEvent`、`AutomationTransformViewState`、`LoopSlot`、`ScrollerTimePreserver`
- PB 值按 RPN 时间线折算成固定设备的 `PitchBendRange`，详见 [MIDI 解析与 Pitch Bend / RPN](midi-and-pitch-bend.md)

一个通道可能有多条源轨，但所有子轨最终都接到同一台乐器。因此通道级 PB/CC 自动化只放在**最长的一条有音符子轨**上，避免第一条源轨较短时后半段自动化被截断。这个选择在 `clone_routed_group` 里完成。

## 空轨与无 Clip 处理

- 旧式单轨模式（Group 里只有一条“乐器 + Clip”轨）遇到空源轨时，会清空设备链，只保留一条空 MIDI 轨，不创建 Clip，也不额外加载乐器。
- 路由模式（1 条乐器轨 + 1 条 MIDI 轨）遇到空源轨时，只保留空 MIDI 轨，不写 Clip。
- 有音符的子轨才会调用 `write_child_clip`；写入后会重新统计 `MidiNoteEvent`，数量不一致会直接报错。

## Tempo 自动化

`set_tempo` 先写 Master/MainTrack 的 `Mixer/Tempo/Manual`，再处理 Tempo 自动化：

- Tempo Map 只有 1 个点时，只写固定 BPM，不创建自动化
- 多个点时，创建或替换 `AutomationEnvelopes` 里目标为 Tempo `AutomationTarget` 的 `AutomationEnvelope`
- 第一个事件是 `-63072000` 的哨兵，保证 Arrangement 起点之前也有值
- 每个变速点写两个同时间的 `FloatEvent`（旧值 → 新值），形成阶跃而不是斜坡
- BPM 超出模板 `MidiControllerRange` 时只给 warning，不修改模板范围

相关代码：`src/als/tempo.rs`，以及 `src/als/inject/clone.rs` 对 `write_child_clip` 的调用。

## 相关阅读

- [转换流水线](pipeline.md)
- [MIDI 解析与 Pitch Bend / RPN](midi-and-pitch-bend.md)
- [转换流程与选项](../workflows/conversion.md)
