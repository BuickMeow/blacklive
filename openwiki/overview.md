---
type: overview
title: 项目总览
description: Blacklive 是一个把标准 MIDI 转换成 Ableton Live .als 工程的 Rust 工具，采用内置模板注入，面向纯钢琴、无 CC 黑乐谱。
tags: [overview, midi, ableton, als, black-midi]
verified:
  - by: openwiki/0.9.0
    at: 2026-11-07T02:37:32.818Z
sources:
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
  - id: openwiki-source-ed8bf05e307c6278442542c2
    resource: repo://src/lib.rs
generated: { by: "opencode", at: "2026-11-07T02:37:32.818Z" }
---

# 项目总览

Blacklive 把标准 MIDI 文件转换成 Ableton Live 的 `.als` 工程。它不是从零生成 `.als`，而是把一个真实 Live 工程当作模板，替换其中的音符、Clip 长度、Tempo 和自动化。

## 输入输出

```text
输入：.mid / .midi
模板：内置 Ac Piano Upright A01 路由模板（或自定义 .als）
输出：按 MIDI 通道分 Group 的 .als
GUI：egui / eframe 0.37
目标：Ableton Live 12.3
```

默认用户流程很短：

```text
[选择 MIDI…]  [导出 .als]
```

输出默认放在 MIDI 同目录、同名 `.als`。模板选择、输出路径、通道/移调/BPM 等都在「高级」里。

## 为什么需要模板

MIDI 只包含演奏数据：

- 音符、力度、时长
- 通道
- Tempo / 拍号
- CC / Pitch Bend / Program Change

`.als` 还包含：

- Live 版本和 XML schema
- `Ac Piano Upright` 的完整设备状态
- Group 结构、路由、Return/Mixer
- 全局 Pointee ID、`NextPointeeId`
- Clip 的视图/循环/自动化状态

所以模板不是“歌曲模板”，而是**工程骨架 + 乐器快照**。同一套模板可以服务所有同音色/同 Live 版本的 MIDI。

## 默认结构

```text
A01 (Group)
├── A01 Inst   (Ac Piano Upright，无 Clip)
├── A01-01     (纯 MIDI 轨，MIDI To = A01 Inst)
└── A01-02 ...
```

- 每个 MIDI 通道一个 Group
- 每个 Group 只有一个钢琴实例
- 同一通道的源轨拆成子轨，全部路由到 `A01 Inst`
- 空源轨保留为空 MIDI 轨，不建 Clip、不加乐器
- Pitch Bend 写在该通道最长有音符子轨的 Clip Envelope 上
- CC 目前只解析/统计，不写入

## 目标定位

当前主要面向：

- 纯钢琴
- 无 CC
- 黑乐谱/高密度 MIDI
- Live 12.3 + `Ac Piano Upright`

不是通用 DAW 转换器，也不是任意音色/任意版本的万能 `.als` 生成器。换音色、换效果、换 Live 版本时需要自定义模板。

## 关键能力

- 模板注入与 Group 克隆
- 每通道一乐器 + MIDI 路由
- 全局 Pointee ID 重映射与校验
- RPN 0,0 动态弯音范围折算
- Tempo 自动化
- 空轨保留
- 内置模板与自定义模板两条路径

## 相关阅读

- [转换流水线](architecture/pipeline.md)
- [转换流程与选项](workflows/conversion.md)
- [当前限制与下一步](limitations.md)
- [快速开始](quickstart.md)
