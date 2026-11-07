---
type: workflow
title: GUI 使用与代码路径
description: Blacklive 的 egui 界面：主流程、高级选项、模板切换、状态日志和错误显示。
tags: [gui, egui, workflow, export, template]
verified:
  - by: openwiki/0.9.0
    at: 2026-11-07T02:37:32.818Z
sources:
  - id: openwiki-source-c990858b6f2147f6305882e9
    resource: repo://src/bin/blacklive-gui/actions.rs
  - id: openwiki-source-ec11794e1c868c63d3e34707
    resource: repo://src/bin/blacklive-gui/app.rs
  - id: openwiki-source-75311a9e6e623d448738b111
    resource: repo://src/bin/blacklive-gui/files.rs
  - id: openwiki-source-1ad8f2eecce01d360fbb56fb
    resource: repo://src/bin/blacklive-gui/status.rs
generated: { by: "opencode", at: "2026-11-07T02:37:32.818Z" }
---

# GUI 使用与代码路径

GUI 是一个很薄的 egui/eframe 壳，核心转换全部调用 library。入口在 `src/bin/blacklive-gui/main.rs`，状态和转换调用在 `app.rs`，文件选择在 `files.rs`。

## 主流程

```text
[选择 MIDI…]  [导出 .als]
```

- 选择 MIDI 后，输出路径自动变为同目录、同名 `.als`
- 也可以把 `.mid` / `.midi` 拖进窗口
- 导出按钮只在 MIDI 路径非空时可用
- 导出成功后出现：
  - 在 Live 中打开
  - 打开所在文件夹

默认使用内置 `Ac Piano Upright` 模板，不需要选模板。

## 高级面板

「高级」默认折叠，里面保留：

- MIDI 路径、模板路径、输出路径
- 使用自定义模板 / 切回内置模板
- 分析模板（Group、轨道、Clip）
- 解析 MIDI（音符、通道、PPQ、BPM、最大复音、CC/PB 数量）
- 通道过滤、移调、BPM 覆盖、最短时值、尾部留白、起点、同音高重叠
- 按源轨道拆分
- 关闭乐器内 Reverb/Delay
- 导出模板 XML（内置或自定义）

## 状态、日志和错误

- 顶部状态行显示当前状态/错误
- 错误用红色
- 日志区显示选择文件、模板分析、MIDI 解析、转换结果和 warning
- warning 以黄色显示
- 转换成功后状态显示输出路径

## 代码路径

```text
main.rs          eframe 启动
app.rs           App 状态、convert()、主 UI、eframe::App
files.rs         文件选择、模板分析、拖拽、show_files/show_template、dump_xml
options.rs       高级选项
stats.rs         MIDI 统计显示
status.rs        状态/日志
actions.rs       FileActions / ActionFlags、打开文件/文件夹
```

`App` 的字段在 `app.rs`，跨模块方法用 `pub(crate)`。主 UI 在 `impl eframe::App for App` 里，通过 `FileActions` 和 `ActionFlags` 把点击事件延迟到 UI 绘制结束后执行。

## 导出流程

`convert()` 会：

1. 读取 MIDI/模板/输出路径
2. 解析 BPM、通道过滤
3. 组装 `ConvertOptions`
4. 调 `convert_midi_to_als` 或 `convert_midi_to_als_with_default_template`
5. 把结果写进 `last_result`
6. 把 warning 写进日志

GUI 不直接操作 XML，也不复制转换逻辑。

## 相关阅读

- [转换流程与选项](conversion.md)
- [构建、测试与 Live 验证](../operations/build-test.md)
- [项目总览](../overview.md)
