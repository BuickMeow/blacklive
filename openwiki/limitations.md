---
type: limitations
title: 当前限制与下一步
description: Blacklive 已实现、未实现、未验证的能力边界，以及下一步可执行的改进。
tags: [limitations, roadmap, cc, polyphony, performance, live]
verified:
  - by: openwiki/0.9.0
    at: 2026-11-07T02:37:32.818Z
sources:
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
  - id: openwiki-source-89b2fadf9596bee25628864e
    resource: repo://src/als/devices.rs
  - id: openwiki-source-71553ff6b7004ea001bf94c7
    resource: repo://src/convert/options.rs
generated: { by: "opencode", at: "2026-11-07T02:37:32.818Z" }
---

# 当前限制与下一步

## 已实现

- 默认内置 A01 路由模板：`Ac Piano Upright` + 一条 MIDI 路由轨 + 干声
- 每个 MIDI 通道一个 Group、一个钢琴实例
- 按源轨拆分子轨，支持空轨保留
- Pitch Bend 包络写入
- RPN 0,0 动态弯音范围折算
- Tempo 自动化
- 内置模板与自定义模板两条路径
- GUI 主流程：选择 MIDI → 导出 `.als`

## 未实现

- **CC 自动化**：CC 只解析/统计，不写入 `.als`
- **Aftertouch**：Channel Pressure / Poly Pressure 未写入
- **Program Change 切换音色**：Program Change 被忽略，不会按音色分轨
- **MIDI Port 路由**：只按 channel 分组，忽略 Port
- **复音上限/voice stealing**：没有在转换器里设置，由乐器自己决定
- **自动试听**：转换器不会启动 Live 或播放输出

## 未验证 / 需要人工确认

- RPN 折算后的实际听感是否完全正确
- 不同钢琴/建模乐器的 PB 听感
- Group 效果器在真实播放中的表现
- 多通道 15 个钢琴实例的 CPU 占用
- 高密度黑 MIDI 在 Live 里的实时播放稳定性

## 兼容性限制

- 当前只保证 **Ableton Live 12.3**
- 自定义模板建议只保留需要的原型 Group，其他 Group/轨道会被保留
- 路由 Group 必须是 1 条乐器轨 + 1 条 MIDI 轨；单轨模式仍兼容但每子轨一个乐器
- 第三方 VST 通常不在 `.als` XML 中暴露 `PitchBendRange`，需要手动设成与转换器相同的固定范围

## 性能限制

- `midly::Smf::parse` 一次性解析整个 MIDI
- 68 万音符可用，但峰值内存约 1.7 GB
- 上千万音符需要惰性解析、流式 XML 或分块处理
- XML 树本身是内存大头；输出 `.als` 的大小不是主要成本

## 下一步

1. **Live 试听**：用 `EnchantedLove` 输出确认 PB/RPN、Group 音量、Tempo。  
2. **CC 映射**：需要一张明确的 CC→效果参数表，再打开 `write_cc_automation`。  
3. **复音策略**：决定是否为 `Ac Piano Upright` 设置最大复音/voice stealing。  
4. **乐器对比**：用单通道 A/B `Ac Piano Upright`、`10-Childhood Home Piano`、`2-E-Piano Dreamy`、`5-Analog Piano`，比较音质与 CPU。  
5. **大文件优化**：如果目标超过百万音符，评估 `quick-xml` 流式写出或分块生成。  
6. **Bitwig 探索**：如果考虑换 DAW，先验证 `Notes to Tracks` 路由和 `The Grid` 建模钢琴。

## 相关阅读

- [关键设计与取舍](decisions.md)
- [构建、测试与 Live 验证](operations/build-test.md)
- [项目总览](overview.md)
