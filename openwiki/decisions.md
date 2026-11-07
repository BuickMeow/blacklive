---
type: decision
title: 关键设计与取舍
description: Blacklive 在模板注入、每通道一乐器、干声、RPN 折算、空轨保留和默认模板来源上的关键决策。
tags: [decisions, architecture, tradeoffs, ableton, midi]
verified:
  - by: openwiki/0.9.0
    at: 2026-11-07T02:37:32.818Z
sources:
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
  - id: openwiki-source-379215f33156a03d017739bd
    resource: repo://src/als/clip.rs
  - id: openwiki-source-96afe7c1afce95b4d368669d
    resource: repo://src/als/inject/clone.rs
  - id: openwiki-source-3f15ae5f13c9890b73329643
    resource: repo://src/convert/grouping.rs
  - id: openwiki-source-71553ff6b7004ea001bf94c7
    resource: repo://src/convert/options.rs
  - id: openwiki-source-b912ad5e2f8c8147e20438c3
    resource: repo://src/convert/pipeline.rs
generated: { by: "opencode", at: "2026-11-07T02:37:32.818Z" }
---

# 关键设计与取舍

这一页记录“为什么这样做”，而不是“代码怎么写”。每条按背景、选择、替代方案、后果组织。

## 1. 保留模板，但从零生成 `.als`

**背景**：MIDI 只描述演奏，不包含 Live 12.3 的 XML schema、`Ac Piano Upright` 设备状态、Group 路由、Return/Mixer、Pointee ID 等。

**选择**：用真实 Live 工程做模板，只替换音符、Clip 长度、Tempo 和 PB/CC。

**替代方案**：

- 从零拼完整 `.als`：需要维护整个 Live schema，风险极高
- 调用 Live API 实时创建：需要 Live 运行，不是无头导出

**后果**：转换器强依赖模板；模板变了，输出音色/路由/效果也会变。但换来的是稳定、可验证的 Live 工程骨架。

## 2. 默认模板内置，用户不用每次选

**背景**：用户不该为每个 MIDI 重新选模板；模板不是“歌曲模板”，而是工程骨架。

**选择**：把默认 A01 模板嵌入二进制，GUI 主流程只保留“选择 MIDI”和“导出 .als”；自定义模板放进高级。

**后果**：默认路径最短；换音色/效果/Live 版本时才需要自定义模板。

## 3. 每通道一个 Group、一个钢琴实例

**背景**：黑乐谱可能有很多源轨；如果每条子轨都加载钢琴，CPU 会爆炸。

**选择**：

- 每个 MIDI 通道一个 Group
- Group 里一条 `A01 Inst` 钢琴轨 + 多条纯 MIDI 子轨
- 所有子轨通过 `MIDI To` 路由到同一台钢琴

**替代方案**：每条源轨一个乐器；或全局只用一台钢琴。

**后果**：

- CPU 不随子轨数增长
- 每个通道可以有自己的 Group 效果/ADSR
- 代价是 15 个通道仍有 15 个钢琴实例

## 4. 默认干声

**背景**：黑乐谱密度高，Reverb/Delay 放在每条子轨会叠加失控。

**选择**：

- 不自动添加 EQ/Compressor/Reverb
- 克隆乐器时关闭内部 Reverb/Delay/Chorus/Echo
- Group/Return 效果保留

**后果**：默认输出是干声；需要空间感时，在 A01/A02 Group 或共享 Return 上手动加。

## 5. 保留空轨

**背景**：有些源轨只有 CC/PB/Program Change，没有音符；直接丢掉会破坏工程结构。

**选择**：

- 有 MIDI 事件但无音符的源轨保留为空 MIDI 轨
- 空轨不建 Clip、不额外加载乐器
- 完全没有 MIDI 事件的 conductor/meta-only 轨不建

**后果**：轨道结构更接近原 MIDI；但空轨不会发声。

## 6. Pitch Bend 放在最长有音符子轨

**背景**：同一通道的 PB 是通道级自动化，只能写在一个 Clip 上；如果放在很短的子轨，后半段 PB 会被截断。

**选择**：在 `automation_child` 里选该通道最长的一条有音符子轨承载 PB/CC。

**后果**：PB 覆盖完整通道；自动化不落在空轨或短轨上。

## 7. CC 暂时只统计不写

**背景**：CC 可能对应踏板、音量、滤波器等不同目标；直接原样写进 Clip 不一定符合目标设备。

**选择**：

- 解析并统计 CC
- GUI 显示每通道 CC 数量
- 转换时给 warning
- `write_cc_automation = false`

**替代方案**：所有 CC 原样写 `ControllerTargets.(n+1)`。这能工作，但会把“CC 到设备参数”的语义留给用户猜。

**后果**：当前输出不包含 CC 自动化；后续需要一张明确的 CC→效果映射表。

## 8. RPN 动态范围折算进 PB 包络

**背景**：MIDI 可以用 RPN 0,0 随时改变 Pitch Bend 范围；Live 的 Simpler 只有一个静态 `PitchBendRange` 参数，不读 RPN。

**选择**：

1. 解析 RPN 0,0
2. 取最大范围（最低 2，最高 48）
3. 把固定范围写进克隆乐器
4. 按 RPN 时间线把 PB 值折算成固定范围下的值

**替代方案**：

- 写 CC101/100/6/38 包络：Simpler 不认
- 用多个乐器分段：太复杂
- 只设一个固定范围：动态 RPN 段落会错

**后果**：PB 包络看起来是固定范围，但听感保留动态范围；第三方插件仍需手动设相同范围。

## 9. 默认模板必须由 Live 自己生成 Group

**背景**：从其他工程直接搬 `GroupTrack` 到目标工程，Live 可能 `map::at` 闪退。

**选择**：在目标 `luyou.als` 里用 Live 自己创建 A01 Group，再整理成内置模板。

**替代方案**：从 Cnstruct 等工程复制 GroupTrack。实际验证会失败。

**后果**：模板必须来自同版本、同工程的 Live 保存结果；换 Live 版本时需要重新生成模板。

## 10. 只验证 Live 12.3

**背景**：`.als` 的 NoteId、Tempo、Group、ClipEnvelope 结构会随 Live 版本变化。

**选择**：当前只保证 Live 12.3。

**后果**：Live 11/10 需要单独模板和回归测试，不是改版本号就能兼容。

## 相关阅读

- [Live 模板与 ID 体系](architecture/template-and-ids.md)
- [MIDI 解析与 Pitch Bend / RPN](architecture/midi-and-pitch-bend.md)
- [当前限制与下一步](limitations.md)
