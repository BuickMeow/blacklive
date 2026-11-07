---
type: quickstart
title: 快速开始
description: 用最短路径构建 Blacklive、选择 MIDI、导出 .als，并找到进一步阅读的页面。
tags: [quickstart, build, run, midi, export]
verified:
  - by: openwiki/0.9.0
    at: 2026-11-07T02:37:32.818Z
sources:
  - id: openwiki-source-23775c3de52f3ab95a13cb8b
    resource: repo://README.md
  - id: openwiki-source-75311a9e6e623d448738b111
    resource: repo://src/bin/blacklive-gui/files.rs
generated: { by: "opencode", at: "2026-11-07T02:37:32.818Z" }
---

# 快速开始

## 1. 构建并启动

```bash
cd /Users/jieneng/Documents/GitHub/blacklive
cargo run --release --bin blacklive
```

## 2. 导出

```text
[选择 MIDI…]  [导出 .als]
```

- 默认使用内置 `Ac Piano Upright` 模板
- 输出默认放在 MIDI 同目录、同名 `.als`
- 也可以把 `.mid` / `.midi` 拖进窗口
- 导出后可以点「在 Live 中打开」或「打开所在文件夹」

## 3. 需要换模板/调参数时

打开「高级」：

- 自定义模板 / 切回内置模板
- 输出路径
- 分析模板
- 解析 MIDI
- 通道、移调、BPM、起点、最短时值、尾部留白、同音高重叠
- 按源轨道拆分
- 关闭乐器内 Reverb/Delay
- 导出模板 XML

## 4. 跑测试

```bash
cargo test
cargo clippy --all-targets
cargo fmt --check
```

真实模板手动测试：

```bash
BLACKLIVE_TEMPLATE=template.als \
BLACKLIVE_MIDI=input.mid \
BLACKLIVE_OUT=output.als \
cargo test --test manual_group -- --ignored --nocapture
```

## 5. 下一步读什么

- 想理解整体流程：[转换流水线](architecture/pipeline.md)
- 想理解 Live 模板/Group/ID：[Live 模板与 ID 体系](architecture/template-and-ids.md)
- 想理解 PB/RPN：[MIDI 解析与 Pitch Bend / RPN](architecture/midi-and-pitch-bend.md)
- 想调参数：[转换流程与选项](workflows/conversion.md)
- 遇到问题：[当前限制与下一步](limitations.md)
