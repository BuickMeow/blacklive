---
type: architecture
title: Live 模板与 ID 体系
description: .als 模板结构、A01 Group、乐器/MIDI 轨路由、Pointee ID 与特殊调制目标的克隆和重映射规则。
tags: [ableton, als, template, group, routing, pointee, ids]
verified:
  - by: openwiki/0.9.0
    at: 2026-11-07T02:37:32.818Z
sources:
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
  - id: openwiki-source-96afe7c1afce95b4d368669d
    resource: repo://src/als/inject/clone.rs
  - id: openwiki-source-542851eef9a58ff083c28e58
    resource: repo://src/als/inject/mod.rs
  - id: openwiki-source-a6252d06236593a45e1db9f2
    resource: repo://src/als/pointee.rs
  - id: openwiki-source-84bdf2d1c84bddd70d0cb582
    resource: repo://src/als/validate.rs
generated: { by: "opencode", at: "2026-11-07T02:37:32.818Z" }
---

# Live 模板与 ID 体系

Blacklive 不从头生成 `.als`，而是把一个真实 Live 工程当模板，克隆其中的 Group 和轨道。模板决定 Live 版本、乐器、设备状态、Group 效果、Return/Mixer 和 XML schema。

## 默认模板结构

```text
A01 (GroupTrack)
├── A01 Inst   (MidiTrack，Ac Piano Upright，无 Clip)
└── A01-01     (MidiTrack，纯 MIDI，占位 Clip，MIDI To = A01 Inst)
```

- 每个 MIDI 通道一个 Group
- 每个 Group 只保留一个钢琴实例
- 同一通道的所有源轨都变成 MIDI 子轨，路由到 `A01 Inst`
- Group 上可以挂该通道的效果器/自动化
- 默认模板来自 Live 在 `luyou.als` 里自己生成的 A01 Group，而不是从别的工程硬搬 GroupTrack

一个通道的 Group 结构在 `src/als/inject/clone.rs` 里克隆，原型解析在 `src/als/inject/mod.rs`。

## 原型模式

`resolve_prototype` 根据 Group 里的 MIDI 子轨数量决定模式：

| 子轨数量 | 模式 | 行为 |
|---:|---|---|
| 1 | Single | 这条轨本身就是“乐器 + Clip”，每条源轨克隆一份乐器 |
| 2 | Routed | 一条乐器轨 + 一条 MIDI 轨，所有源轨共享这台乐器 |
| 其它 | 报错 | Group 结构不符合要求 |

路由模式还会检查：

- 原型 Clip 必须在 MIDI 轨上，不能在乐器轨上
- MIDI 轨的 `MidiOutputRouting/Target` 必须指向同 Group 的乐器轨
- 乐器轨必须有 `Id`

## 路由模式克隆

`clone_routed_group` 做这些事：

1. 克隆乐器轨，重映射 Pointee ID，分配新 Track Id
2. 改名为 `A01 Inst`
3. 设置 `TrackGroupId` 到新 Group
4. `AudioOutputRouting` 指向 `AudioOut/GroupTrack`
5. 关闭录音 arm，按需关闭内部 Reverb/Delay
6. 写入固定 `PitchBendRange`
7. 清空原型 Clip
8. 对每条源轨克隆 MIDI 轨：
   - 新 Track Id
   - `MidiOutputRouting/Target = MidiOut/Track.<instrument_id>/TrackIn`
   - 空轨只保留轨道，不建 Clip
   - 有音符的轨写 Clip、音符、PB/CC

## ID 体系

`.als` 中有一类全局 ID，必须在整个工程里唯一：

- `Pointee`
- `AutomationTarget`
- `ModulationTarget`
- `ControllerTargets.*`
- `*AutomationTarget` / `*ModulationTarget` 后缀的特殊目标

`PointeeAllocator` 从模板的 `NextPointeeId` 和实际最大 ID 中取最大值，再从 `max + 1` 开始分配。克隆每个 Group/Track 时：

1. 收集该子树里所有 Pointee ID
2. 给每个旧 ID 分配新 ID
3. 重写所有 `PointeeId Value=...` 引用
4. 最后把 `NextPointeeId` 推进到分配器末尾

## 为什么特殊 ModulationTarget 也要重映射

Live 12 的 `FreezeSequencer` 里有：

- `VolumeModulationTarget`
- `TranspositionModulationTarget`
- `GrainSizeModulationTarget`
- `FluxModulationTarget`
- `SampleOffsetModulationTarget`
- `ComplexPro...ModulationTarget`
- `TransientEnvelopeModulationTarget`

它们也属于全局 Pointee ID。早期只重映射普通 `AutomationTarget` / `ModulationTarget`，多通道克隆后这些 ID 会重复，Live 会报：

```text
PointeeId 16127 is used 15 times.
```

现在 `is_pointee_holder` 用后缀匹配：

```rust
element.name == "Pointee"
    || element.name.ends_with("AutomationTarget")
    || element.name.ends_with("ModulationTarget")
    || element.name.starts_with("ControllerTargets.")
```

这样普通目标和特殊目标都会参与重映射。

## 校验

`validate_live_set` 做两层检查：

- 同一父节点下不能有重复 `Id`
- 全局 Pointee/AutomationTarget/ModulationTarget/ControllerTargets 不能重复
- 最大 Pointee ID 必须小于 `NextPointeeId`

输出在写盘前会重新解析一次再校验，避免序列化后 ID 变化。

## 为什么默认模板要由 Live 自己生成 Group

从另一个工程直接搬 `GroupTrack` 到当前工程，即使 XML 看起来完整，Live 也可能在加载时触发：

```text
FatalError: [std::out_of_range] map::at: key not found
```

原因是 GroupTrack 内部引用了当前工程的对象映射（设备、路由、FreezeSequencer、Mixer 等），不是孤立的 XML。最稳的做法是让 Live 在目标工程里自己创建 Group，再把这套 Group 作为模板。默认模板就是这样做出来的。

## 相关阅读

- [转换流水线](pipeline.md)
- [关键设计与取舍](../decisions.md)
- [当前限制与下一步](../limitations.md)
