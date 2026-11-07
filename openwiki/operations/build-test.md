---
type: operations
title: 构建、测试与 Live 验证
description: 如何构建 GUI、跑自动测试、做真实 MIDI 转换，并在 Ableton Live 12.3.8 中验证输出。
tags: [build, cargo, test, live, verification, debugging]
verified:
  - by: openwiki/0.9.0
    at: 2026-11-07T02:37:32.818Z
sources:
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
  - id: openwiki-source-b912ad5e2f8c8147e20438c3
    resource: repo://src/convert/pipeline.rs
  - id: openwiki-source-80837d304971eaed07dd196d
    resource: repo://tests/embedded_template.rs
  - id: openwiki-source-b5cad31237effa37a1d0b8d8
    resource: repo://tests/manual_group.rs
generated: { by: "opencode", at: "2026-11-07T02:37:32.818Z" }
---

# 构建、测试与 Live 验证

## 构建与运行

```bash
cd /Users/jieneng/Documents/GitHub/blacklive
cargo run --release --bin blacklive
```

GUI 主流程：

```text
[选择 MIDI…]  [导出 .als]
```

默认使用内置 `Ac Piano Upright` 模板，输出默认放在 MIDI 同目录、同名 `.als`。

## 自动测试

```bash
cargo test
cargo clippy --all-targets
cargo fmt --check
```

当前测试分成两层：

### 1. 内置模板集成测试

`tests/embedded_template.rs` 覆盖：

- 内置模板是干净的 A01 路由 Group
- Group 里有 1 条乐器轨 + 1 条 MIDI 轨，且 MIDI 轨路由到乐器轨
- 纯钢琴多通道 MIDI 能正确分组
- 空源轨保留、meta-only 轨跳过
- Pitch Bend 写在最长有音符子轨，而不是短轨或空轨

### 2. 手动真实模板测试

`tests/manual_group.rs` 是 `#[ignore]` 测试，用环境变量指定真实模板和 MIDI：

```bash
BLACKLIVE_TEMPLATE=template.als \
BLACKLIVE_MIDI=input.mid \
BLACKLIVE_OUT=output.als \
cargo test --test manual_group -- --ignored --nocapture
```

它会：

- 打印模板里的 Clip
- 选择 Group 内 Clip
- 跑完整转换
- 打印每个 Group/子轨的音符数
- 再次检查输出模板并核对总音符数

## 输出自检

转换器在写盘前做两次校验：

1. 注入后 `validate_live_set(&root)`
2. 序列化后重新解析，再 `validate_live_set(&reparsed)`

然后 `write_als_atomic` 用 gzip + 临时文件 + rename 写出。

你也可以手动检查：

```bash
gzip -dc output.als > output.xml
grep -n '<GroupTrack' output.xml
grep -n '<MidiClip' output.xml
grep -n '<MidiNoteEvent' output.xml | head
```

## 在 Live 12.3.8 中验证

已验证过的加载路径：

- 内置模板本身
- 单通道 `satpiano` 输出
- 15 通道 `EnchantedLove` 输出
- RPN 动态范围折算后的 `EnchantedLove` 输出

建议的试听检查项：

1. 音色是否是 `Ac Piano Upright`
2. 力度/时值是否正常
3. Pitch Bend 是否在 2/6/12/24 半音段落正确
4. Group 音量/效果是否合理
5. Tempo 自动化是否跟随
6. 空轨是否保留但不发声

## 常见问题

### 输出在 Live 里打不开

先确认：

- 模板是否来自同版本 Live
- 是否有重复 Pointee ID
- `NextPointeeId` 是否大于所有 Pointee ID
- Group 是否由 Live 在当前工程里生成

### 有 CC 但输出没有 CC 自动化

这是当前设计：`write_cc_automation = false`。CC 只统计，不写入。

### Pitch Bend 听感不对

检查：

- 输出乐器的 `PitchBendRange`
- 是否用了第三方插件且没有手动设相同范围
- RPN 动态范围是否被正确折算

## 相关阅读

- [当前限制与下一步](../limitations.md)
- [GUI 使用与代码路径](../workflows/gui.md)
- [转换流水线](../architecture/pipeline.md)
