# Blacklive

把 MIDI 转换成 Ableton Live `.als` 工程，采用 **模板注入**，不从头生成 `.als`。

- 输入：标准 MIDI（`.mid` / `.midi`）
- 模板：默认内置干净的 `Ac Piano Upright` 路由模板；需要换音色/效果/Live 版本时才用自定义模板
- 输出：按 MIDI 通道分 Group 的 Live 工程
- GUI：egui / eframe **0.37**，复刻 Ableton Live 12 的编曲/会话界面（Material Icons + egui_extras）
- 目标规模：纯钢琴、无 CC 黑乐谱；已实测 68 万音符、15 个通道组
- 仓库内文档：`openwiki/quickstart.md`（OpenWiki 生成）

> 已用真实 Live 12.3 工程模板与真实 MIDI 跑通：Group 克隆、子轨克隆、全局 ID 重映射、Pitch Bend Clip 自动化、Tempo 自动化、空轨保留均通过 XML/gzip 自检。尚未在 Live 里实际点开试听；最后一步需要你打开输出验证音色、力度和 Group 效果。

---

## 1. 结构

转换结果按 MIDI 通道分组。推荐的路由模式：

```text
A01 (Group)
├── A01 Inst   (MIDI Track，Ac Piano Upright，无 Clip)
├── A01-01     (MIDI Track，无乐器，Arrangement Clip，MIDI To = A01 Inst)
└── A01-02     (MIDI Track，无乐器，Arrangement Clip，MIDI To = A01 Inst)

A02 (Group)
├── A02 Inst
└── A02-01 ...
```

- 每个 MIDI 通道生成一个 **GroupTrack**，命名 `A01`、`A02`……
- 同一通道内，每个源 MIDI 轨道生成一条**纯 MIDI 子轨**；该通道所有子轨都在同一个 Group 里。
- 有 MIDI 事件但没有音符的源轨会保留为空 MIDI 轨（无 Clip、无乐器），不会丢轨。
- 每个 Group 只有一个钢琴实例 `A01 Inst`，所有 MIDI 子轨都路由到它。
- Group 从模板里的原型 Group 克隆，Group 的自动化、效果器、Mixer 设置都会保留。
- 如果模板里已经有名为 `A01`、`A02`… 的 Group，会优先用对应 Group 作为该通道的原型；没有则使用界面里选中的默认原型。这样可以给不同通道不同乐器/效果。
- MIDI 子轨的音频输出不重要（它们没有乐器）；`A01 Inst` 的音频输出指向 Group。
- 每个 MIDI 子轨生成一个 Arrangement Clip。
- Pitch Bend 写在最长的一条有音符 MIDI 子轨的 Clip Envelope 里，对整个通道共用一台钢琴生效。MIDI CC 暂时只解析/统计，不写入；后续再做 CC→效果映射。
- MIDI Tempo Map 写成 Master Tempo 自动化。

内置默认模板就是这套结构：一个 `A01` Group，里面是 `Ac Piano Upright` 乐器轨 + 一条路由 MIDI 轨，干声，无 CC 依赖。程序会按 MIDI 的通道数克隆出 `A01`、`A02`……

也兼容旧式单轨模式：Group 里只有一条“乐器 + Clip”轨道时，转换器会按源轨道克隆这条轨（每子轨一个乐器）。

默认 `按源轨道拆分` 开启。如果你想把同一通道的所有源轨道合并成一条子轨，可以在界面关闭它。

---

## 2. 模板

默认**不需要手动选模板**。程序内置 `assets/default.als`：

```text
A01 (Group，干声)
├── A01 Inst   (Ac Piano Upright，无 Clip)
└── A01-01     (纯 MIDI 轨，空占位 Clip，MIDI To = A01 Inst)
```

内置模板保留 Live 12.3 的工程骨架、`Ac Piano Upright` 设备状态、Group/Mixer/Return/ID 体系。转换时只替换音符、Clip 长度、Tempo 和 Pitch Bend，所以任何纯钢琴、无 CC 的 MIDI 都可以直接导出。

只有这些情况才需要自定义模板：

- 换钢琴/音色，或调整 `Ac Piano Upright` 的 ADSR/音色
- 给不同通道挂不同 Group 效果器
- 换 Live 版本
- 需要自定义 Return/Send/Master 状态

自定义模板会保留其中的 Live 版本、钢琴设备、Group 效果器、自动化、Mixer 设置。

推荐使用**路由模板**：

1. 用**目标 Ableton Live 版本**新建一个空工程。
2. 删除不需要的轨道和效果器，只保留 Master / Return。
3. 创建一条乐器轨 `A01 Inst`，加载 `Ac Piano Upright`（或你微调后的钢琴），不要放 Clip。
4. 创建一条纯 MIDI 轨 `A01-01`，不要加载乐器，放一个短占位 Clip（Session 或 Arrangement 都可以），并把它的 `MIDI To` 指向 `A01 Inst`。
5. 选中这两条轨道，`Cmd+G` 创建 Group，命名为 `A01`。
6. 把该通道需要的效果器/自动化挂在 **Group A01** 上；Reverb 建议放在 Group 或共享 Return，不要留在每条子轨。
7. 保存为 `template.als`，关闭 Live。
8. 如果希望每个通道使用不同钢琴/Group 效果，再做 `A02`、`A03`… 各自一个路由 Group。
9. 如果模板里有其他 Group/MIDI 轨道，转换器会保留它们；建议模板只保留需要的原型 Group。

> 路由 Group 需要 **1 条乐器轨（无 Clip） + 1 条 MIDI 轨（有占位 Clip，MIDI To 指向乐器轨）**。
> 旧式模板也可以：Group 里只有 1 条“乐器 + Clip”轨；转换器会退化为每子轨一个乐器。
> 可选：`A01`、`A02`… 每个通道一个原型 Group；未提供对应名称时使用默认原型。
> 内置模板由 Live 在 `luyou.als` 里生成的 `A01` Group 整理而来；源码里的 `assets/default.als` 就是它，已实测 Live 12.3.8 能直接加载。
> 转换器默认关闭原型乐器内部的 Reverb；如果要保留，高级里关掉「关闭乐器内 Reverb/Delay」。

采样率是 Live 的全局 Preferences，不保存在 `.als` 里。请先在 Live 的 Audio 设置里设好。

### 检查模板 XML

```bash
gunzip -c template.als > template.xml
grep -n '<GroupTrack' template.xml
grep -n '<MidiClip' template.xml
grep -n '<MidiNoteEvent' template.xml | head
```

Live 12.3 的音符结构：

```xml
<Notes>
  <KeyTracks>
    <KeyTrack Id="0">
      <Notes>
        <MidiNoteEvent Time="0" Duration="0.5" Velocity="100" OffVelocity="64" NoteId="1" />
      </Notes>
      <MidiKey Value="60" />
    </KeyTrack>
  </KeyTracks>
  <NoteIdGenerator><NextId Value="2" /></NoteIdGenerator>
</Notes>
```

---

## 3. 运行

```bash
cd /Users/jieneng/Documents/GitHub/blacklive
cargo run --release --bin blacklive
```

界面按 Ableton Live 12 的布局复刻，打开后直接是工程视图：

```text
┌ 标题栏 ─────────────────────────────────────────────────────┐
│ 文件菜单 / 视图切换 / 传输控制 / BPM / 拍号 / 选择 MIDI / 导出 .als │
├ 浏览器 ─────────────┬ 编曲（Arrangement）或会话（Session）──────┤
│ 当前工程、MIDI、模板、 │ 轨道、Clip、轨道头、返回/Master、时间尺    │
│ 输出、分析、日志       │                                         │
├ 转换器 ─────────────┴─────────────────────────────────────────┤
│ BPM / 移调 / 通道 / 最短时值 / 尾部 / 起点 / 拆分 / 效果开关      │
└ 状态栏 ───────────────────────────────────────────────────────┘
```

主流程：

```text
[选择 MIDI…]  [导出 .als]
```

- 默认使用内置 `Ac Piano Upright` 模板，不用选模板。
- 输出默认放在 MIDI 同目录、同名的 `.als`。
- 拖拽 `.mid` / `.midi` 到窗口即可。
- 左上角两个方块在 **编曲（Arrangement）** 和 **会话（Session）** 之间切换。
- 顶部 BPM 字段可以点击输入；清空后跟随 MIDI/模板 BPM。旁边的 `Tap` 支持点拍测速。
- 左侧浏览器「当前工程」里可以直接选择 MIDI、模板、输出路径，也可以触发解析/分析。
- 底部「转换」是参数面板，「信息」显示 MIDI/模板/转换统计，「日志」显示完整日志。
- 导出后可直接「在 Live 中打开」或「打开所在文件夹」。

> 界面只是 Live 风格的壳；音符、Group、Pitch Bend、Tempo 的转换逻辑没有变化。没有 MIDI 时，编曲区会显示与 Live 一致的空白工程和提示文字。

---

## 4. 转换逻辑

### MIDI tick → beat

```text
beat = tick / ppq
```

Ableton 的 `Time` / `Duration` 使用四分音符 beat。PPQ 从 MIDI 头读取，不是固定 96。

### 音符

- 按音高分组为 `KeyTrack`，每个 `MidiKey` 一组。
- `MidiNoteEvent` 写入 `Time`、`Duration`、`Velocity`、`OffVelocity`。
- 模板有 `NoteIdGenerator` 时写 `NoteId` 并更新 `NextId`；没有则省略，兼容旧模板。
- 同音高重叠默认保留，可勾选截短前音。
- 没有 Note Off 的音符丢弃，避免超长挂音。

### Pitch Bend 自动化

- MIDI CC 仍然解析并按通道统计，界面显示数量；暂时不写 Clip Envelope，转换时会提示 CC 未写入。
- `ControllerTargets.N` 的 ID 在克隆子轨时会重映射；ClipEnvelope 指向重映射后的 ID。
- Pitch Bend 使用 `ControllerTargets.0`，按 Live 的 -8192..8191（中心 0）写线性包络。
- 已用 `luyou.als` 的 0→8191→0 PB Envelope 对照：target、sentinel、时间点和值范围一致。
- 解析 RPN 0,0（Pitch Bend Sensitivity）：CC101/100 选择 RPN，CC6/38 写入半音/音分。
- 动态 RPN 范围会按时间线折算进 PB 包络：固定范围取 MIDI 中出现的最大 RPN（最低 2，最高 48 半音）；没有 RPN 时按标准 ±2 半音。
- 克隆乐器时会把 Live 乐器的 `PitchBendRange` / `PitchbendRange` 参数设成同一个固定范围；第三方插件需要在插件里手动设成相同值。
- Live 的 Simpler 不直接读 RPN，所以不写 CC101/100/6/38 包络，而是把变化烘焙进 PB 值。
- 当前 **不写 Channel Pressure / Poly Pressure**（Aftertouch 按你的要求不做）。
- Pitch Bend 只写在一条 MIDI 子轨上；选择该通道里最长的有音符子轨，避免第一条源轨较短时后半段 PB 被截断。其它子轨只带音符。

### Group / 子轨克隆

- 原型 Group 克隆一次，得到该通道的 Group。
- 原型 MIDI 子轨按源轨道克隆多次，全部指向新 Group。
- Track Id、GroupTrack Id 重新分配。
- `AutomationTarget`、`*ModulationTarget`、`Pointee`、`ControllerTargets.*` 的全局 ID 以及 `PointeeId` 引用统一重映射。
- `FreezeSequencer` 下的 `VolumeModulationTarget`、`GrainSizeModulationTarget` 等特殊调制目标也按全局 Pointee ID 处理，避免多轨克隆后重复。
- `NextPointeeId` 推进到新 ID 之后。
- `A01 Inst` 的 `AudioOutputRouting` 指向 `AudioOut/GroupTrack`；纯 MIDI 子轨没有乐器，音频输出不重要。
- 原型 Group/子轨从输出中移除。
- 子轨 ClipSlot 中原有 Clip 清空，只保留新生成的 Arrangement Clip。

### Tempo / Clip 长度

- `CurrentEnd = start + max(note end)`
- `LoopOn = false`
- Arrangement Clip 的 `Time = start`（默认 0）
- MIDI Tempo Map 写入 `MainTrack/AutomationEnvelopes` 的 Tempo 自动化。
- 用户指定 `start` 时，Tempo 自动化时间同步偏移。
- 使用固定 BPM 覆盖时，清空模板原有 Tempo 自动化并只保留固定值。

### 输出安全

- 写入前重新解析 XML 自检。
- 先写同目录临时文件，再 `rename` 到目标 `.als`。
- 保留模板根版本、设备、Group 效果、轨道、Master 和未知节点。

---

## 5. 项目结构

```text
blacklive/
├── Cargo.toml
├── README.md
├── assets/
│   └── default.als         # 内置 A01 路由模板（Ac Piano Upright，干声）
├── src/
│   ├── lib.rs              # 对外 API、内置模板、数字格式化
│   ├── midi/               # MIDI 类型、解析、RPN、Tempo、统计
│   ├── als/                # gzip/XML、模板检查、ID 重映射、Group 克隆、Clip/PB/Tempo
│   ├── convert/            # 通道分组、过滤/移调、转换编排
│   └── bin/
│       └── blacklive-gui/  # Live 风格 eframe/egui 界面（主题、编曲、会话、浏览器、转换器）
│           ├── main.rs
│           ├── app.rs
│           ├── files.rs
│           ├── options.rs
│           ├── stats.rs
│           ├── status.rs
│           └── actions.rs
└── tests/
    ├── embedded_template.rs   # 内置模板 + 纯钢琴多通道/空轨/PB 回归测试
    ├── fixtures/pure_piano.mid
    ├── fixtures/pb_longest_track.mid
    └── manual_group.rs        # 真实工程 Group 克隆手动集成测试
```

核心 API：

```rust
// 默认：内置模板
pub fn convert_midi_to_als_with_default_template(
    midi_path: &Path,
    output_path: &Path,
    opts: &ConvertOptions,
) -> Result<ConversionResult>;

// 自定义模板
pub fn convert_midi_to_als(
    midi_path: &Path,
    template_path: &Path,
    output_path: &Path,
    opts: &ConvertOptions,
) -> Result<ConversionResult>;
```

```rust
pub fn inspect_template(path: &Path) -> Result<TemplateReport>;
pub fn inspect_default_template() -> Result<TemplateReport>;
```

---

## 6. 版本兼容

目前**只在 Live 12.3 模板上验证**。代码是模板驱动的，但不同 Live 版本仍有以下差异：

- `MidiNoteEvent` 是否有 `NoteId`
- `Notes` 下的 `PerNoteEventStore` / `NoteProbabilityGroups` / `NoteIdGenerator`
- `MidiClip` 的 `TakeId` / `CurrentStart` / `Loop` 字段
- Tempo 自动化的 `AutomationEnvelope` 结构
- GroupTrack 的 `Slots`、Mixer、设备 ID 体系
- `ControllerTargets` 与 ClipEnvelope 的结构

建议：

- **自用只做 12.3**：最稳。
- **兼容 12.0–12.3**：通常可以用对应版本保存的模板；建议先跑小样。
- **兼容 Live 11**：需要一个 Live 11 模板做回归，重点检查 NoteId/Tempo/Group。
- **兼容 Live 10**：不是改一个版本号就行；Group、Clip envelope、Tempo 自动化都有结构差异。需要 Live 10 模板 + 单独适配与测试。若只是自己用 12，不建议为 10 增加维护面。

代码已做部分兼容保护：没有 `NoteIdGenerator` 就不写 `NoteId`；没有 Tempo `AutomationTarget` 就退回固定 BPM；没有 Group 就报错而不是生成不可用工程。

---

## 7. 黑乐谱性能建议

- 优先用采样钢琴：`11-Ac Piano Upright` 是当前 EmptyPiano 里最轻的（单个 `OriginalSimpler` + 一个 Reverb），适合做默认原型。
- 电钢可用 `9-E-Piano Straight`（单个 Operator + Reverb/Glue），CPU 很轻。
- `6-E-Piano Basic`、`12-E-Piano Old School` 属于中等；多链/效果会叠加。
- `2-E-Piano Dreamy`（Collision）、`5-Analog Piano`（Drift）、`8-Cracked Piano`（Meld）是物理建模/合成器，单 voice 成本更高。
- `10-Childhood Home Piano`（3 个 MultiSampler、92 个 SampleRef、踏板/按键噪声、混响）、`7-Abyss Keys`（双层 Wavetable + 效果）、`13-Exo Keys`（双层 Wavetable + Delay/Reverb）在 1 万音符以上会明显更吃 CPU。
- 每个 Group 包含该通道的所有源 MIDI 子轨，但只保留一个钢琴实例 `A01 Inst`；所有 MIDI 子轨通过 `MIDI To` 路由到它。这样 CPU 不随子轨数增长。
- 旧式模板（每条子轨一个乐器）仍兼容，但不推荐高密度黑乐谱使用。
- 黑乐谱优先把 Reverb/Delay 放到 Group 或共享 Return，不要在每条子轨里各克隆一个。转换器默认会关闭原型乐器内部的 Reverb/Delay/Chorus/Echo，Group/Return 效果保留。
- 当前默认是**干声**：不自动添加 EQ/Compressor/Reverb；需要时在 A01/A02 Group 或 Return 上手动加。
- 建议给钢琴设置合适的复音上限/voice stealing，并考虑在最终播放时 freeze/flatten。
- 如果所有通道都是 program 0，用一个默认钢琴原型就够；需要每通道不同音色时再做 A01/A02 原型。

## 8. 当前限制

- 默认模板固定为 Live 12.3 + `Ac Piano Upright` + 干声；换音色/效果/版本需要自定义模板。
- 路由 Group 需要 1 条乐器轨 + 1 条 MIDI 轨；旧式单轨 Group 仍兼容。
- 可以按 `A01`、`A02`… 提供不同原型 Group；未提供的通道使用默认原型。
- 只写 Pitch Bend 自动化；MIDI CC 暂时只解析/统计、不写入；Channel Pressure / Poly Pressure 未写入。
- RPN 动态范围已折算进 PB 包络，但最终弯音听感仍建议在 Live 里试听确认。
- Pitch Bend 已按 `luyou.als` 的 `.0 / -8192..8191` 对照；仍建议在 Live 里试听一次。
- Program Change 被忽略；不会按音色切换分轨。
- MIDI Port 被忽略；只按 channel 分组。
- 自定义模板里其他 Group/轨道会保留，建议使用干净模板。
- 有 MIDI 事件但无音符的源轨会保留为空 MIDI 轨；完全没有 MIDI 事件的 conductor/meta-only 轨不会建。
- 当前用 `midly::Smf::parse` 一次性解析；68 万音符可用，但峰值内存约 1.7 GB，上千万音符需要改惰性解析/流式 XML。
- 尚未在 Live 内自动试听；需要你打开输出验证音色、力度、踏板和 Group 效果。

---

## 9. 已验证

- 内置模板：Live 自己生成的 A01 路由 Group、1 条 `Ac Piano Upright` 乐器轨 + 1 条 MIDI 轨、无音符占位 Clip、Group 干声、Return/Send 数量与模板一致。
- 内置模板转换：纯钢琴多通道 / 多源轨 / 空轨保留 / meta-only 轨跳过，XML/gzip 与 ID 自检通过。
- Live 12.3.8 实测打开：内置模板、单通道 `satpiano` 输出、15 通道 `EnchantedLove` 输出均可正常加载。
- RPN 动态范围测试：`EnchantedLove` 的 2/4/6/12/24 半音变化会折算进 PB 包络，输出乐器的 `PitchBendRange` 统一为 24 半音，Live 可正常加载。
- 早期从其他工程克隆 GroupTrack 会导致 Live `map::at` 闪退，已改为使用 Live 在当前工程里生成的 Group。
- 多通道克隆：`FreezeSequencer` 里的 `*ModulationTarget` 也参与全局 ID 重映射；15 通道工程输出无重复 PointeeId。
- 真实 Live 12.3 工程模板 + 真实 MIDI：Group/子轨克隆、全局 ID 重映射、Arrangement Clip 注入、MIDI 路由、Pitch Bend Clip 自动化、Tempo 自动化，XML/gzip 校验通过。
- 单通道 1,228 音符：输出约 184 KB `.als`。
- 68 万音符黑 MIDI（5 MB 输入、15 个通道组）：release 下约 3.3 秒转换，输出约 4.2 MB `.als`，峰值内存约 1.7 GB。
- Group 效果器克隆测试：原型 Group 的 DrumBuss / MultibandDynamics / Eq8 被完整克隆；输出无重复全局 ID，所有 `PointeeId` 引用可解析。
- A01/A02 不同原型测试：A01 使用 Operator 链，A02 使用 Drift 链；输出正确按通道选择不同原型，ID 校验通过。
- `cargo test`：12 个单元测试 + 3 个内置模板集成测试通过；另有 1 个手动 Group 集成测试（`--ignored`）。

手动集成测试：

```bash
BLACKLIVE_TEMPLATE=template.als \
BLACKLIVE_MIDI=input.mid \
BLACKLIVE_OUT=output.als \
cargo test --test manual_group -- --ignored --nocapture
```

---

## 10. 下一步 / 待确认

1. **试听**：请在 Live 12.3 打开一个输出，确认音色、力度、Pitch Bend、Group 音量。
2. **复音上限**：是否给 `Ac Piano Upright` 设置 voice stealing / 最大复音？当前未限制。
3. **CC 映射**：CC 现在只统计不写入；下一步需要你给具体映射（例如 CC1 → 某参数、CC64 → 踏板、CC11 → 音量）。
4. **每通道效果**：目前所有通道都用同一套 A01 原型、干声；如果需要 A02/A03 不同效果，再做对应原型 Group。
5. **钢琴微调**：`Ac Piano Upright` 的 ADSR/音色如果还要改，直接替换 `assets/default.als` 即可，转换器代码不用动。
6. **同通道多轨**：默认每个源 MIDI 轨道一条子轨；如果黑乐谱轨数过多，可以关闭「按源轨道拆分」合并。

---

## License

MIT OR Apache-2.0
