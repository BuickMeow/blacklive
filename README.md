# Blacklive

把 MIDI 转换成 Ableton Live `.als` 工程，采用 **模板注入**，不从头生成 `.als`。

- 输入：标准 MIDI（`.mid` / `.midi`）
- 模板：用户在自己的 Ableton Live 版本里保存的干净工程
- 输出：按 MIDI 通道分 Group 的 Live 工程
- GUI：egui / eframe **0.37**
- 目标规模：约 1 万音符；已验证 5.4 万音符黑乐谱 MIDI

> 已用真实 Live 12.3 工程模板与真实 MIDI 跑通：Group 克隆、子轨克隆、全局 ID 重映射、CC64 Clip 自动化、Tempo 自动化均通过 XML/gzip 自检。尚未在 Live 里实际点开试听；最后一步需要你的 Grand Piano 模板。

---

## 1. 结构

转换结果按 MIDI 通道分组：

```text
A01 (Group)
├── 源 MIDI 轨道 1 (MIDI Track，Grand Piano)
└── 源 MIDI 轨道 2 (MIDI Track，Grand Piano)

A02 (Group)
└── 源 MIDI 轨道 1 (MIDI Track，Grand Piano)
```

- 每个 MIDI 通道生成一个 **GroupTrack**，命名 `A01`、`A02`……
- 同一通道内，每个源 MIDI 轨道生成一条子 MIDI 轨道。
- 所有子轨都从模板里的原型 MIDI 轨克隆，因此使用同一个乐器/设备链。
- Group 从模板里的原型 Group 克隆，Group 的自动化、效果器、Mixer 设置都会保留。
- 子轨的音频输出指向所属 Group。
- 每个子轨生成一个 Arrangement Clip。
- MIDI CC（包括 CC64 踏板）写成 Clip Envelope 自动化。
- MIDI Tempo Map 写成 Master Tempo 自动化。

默认 `按源轨道拆分` 开启。如果你想把同一通道的所有源轨道合并成一条子轨，可以在界面关闭它。

---

## 2. 模板要求

模板是整个方案的关键。模板里的 Live 版本、Grand Piano 设备、Group 效果器、自动化、Mixer 设置都会被保留。

1. 用**目标 Ableton Live 版本**新建一个空工程。
2. 删除不需要的轨道和效果器，只保留 Master / Return 和下面这个原型 Group。
3. 创建一个 **Group**，命名为 `A01`。
4. 在 Group 里放**一条 MIDI 轨道**，加载 Live 自带的 **Grand Piano**。
5. 在这条 MIDI 轨道上放一个短占位 Clip（Session 或 Arrangement 都可以，长度 1 小节即可）。
6. 如果希望每个通道使用 Group 效果器/自动化，把效果器和自动化挂在**原型 Group** 上。
7. 保存为 `template.als`，关闭 Live。
8. 如果模板里有其他 Group/MIDI 轨道，转换器会保留它们；建议模板只保留一个原型 Group 以得到干净输出。

> 原型 Group 下必须**恰好有一条 MIDI 轨道**作为子轨原型。转换器会按通道和源轨道克隆这条子轨。

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

界面支持：

- 选择 MIDI / 模板 `.als` / 输出 `.als`
- 拖拽 `.mid` / `.als` 到窗口
- 分析模板，列出 Group、轨道、Clip
- 解析 MIDI，显示音符数、通道、PPQ、BPM、最大复音、变速点、CC 数量
- 通道过滤、移调、BPM 覆盖、最短时值、尾部留白、起点、同音高重叠处理
- 按源轨道拆分、CC 自动化开关
- 转换后在 Live 中打开、打开输出文件夹

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

### CC 自动化

- 解析所有 MIDI CC（0–127），按通道保存。
- 每个子轨 Clip 写入对应的 `ClipEnvelope` 自动化。
- `ControllerTargets.N` 的 ID 在克隆子轨时会重映射；ClipEnvelope 指向重映射后的 ID。
- 同一时间点的重复 CC 只保留最后一个；值不变时不重复写事件。
- CC 变化用同一时间点的两个 `FloatEvent`（旧值 + 新值）写成阶跃，避免 Live 插值成渐变。
- 当前 **不写 Pitch Bend / Channel Pressure / Poly Pressure**；只写 CC。

### Group / 子轨克隆

- 原型 Group 克隆一次，得到该通道的 Group。
- 原型 MIDI 子轨按源轨道克隆多次，全部指向新 Group。
- Track Id、GroupTrack Id 重新分配。
- `AutomationTarget`、`ModulationTarget`、`Pointee`、`ControllerTargets.*` 的全局 ID 以及 `PointeeId` 引用统一重映射。
- `NextPointeeId` 推进到新 ID 之后。
- 子轨 `AudioOutputRouting` 指向 `AudioOut/GroupTrack`。
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
├── src/
│   ├── lib.rs              # 对外 API、数字格式化
│   ├── midi.rs             # MIDI 解析、LIFO 配对、Tempo Map、CC 事件
│   ├── als.rs              # gzip/XML、Group 克隆、ID 重映射、音符/CC/Tempo 注入
│   ├── convert.rs          # 通道分组、过滤/移调、转换编排
│   └── bin/
│       └── blacklive-gui.rs   # eframe/egui 0.37 桌面界面
└── tests/
    └── manual_group.rs     # 真实工程 Group 克隆手动集成测试
```

核心 API：

```rust
pub fn convert_midi_to_als(
    midi_path: &Path,
    template_path: &Path,
    output_path: &Path,
    opts: &ConvertOptions,
) -> Result<ConversionResult>;
```

```rust
pub fn inspect_template(path: &Path) -> Result<TemplateReport>;
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

## 7. 当前限制

- 原型 Group 下必须恰好一条 MIDI 轨道；多原型/按通道不同乐器尚未实现。
- 所有通道默认使用同一个原型 Group 的效果器/自动化；如果每个通道需要不同效果链，需要后续支持“按 A01/A02 选择不同原型 Group”。
- 只写 MIDI CC 自动化；Pitch Bend / Channel Pressure / Poly Pressure 尚未写入。
- Program Change 被忽略；不会按音色切换分轨。
- MIDI Port 被忽略；只按 channel 分组。
- 模板里其他 Group/轨道会保留，建议使用干净模板。
- 当前用 `midly::Smf::parse` 一次性解析；1 万音符没问题，上亿音符需要复用 yinhe 的惰性解析。
- 尚未在 Live 内自动试听；需要你打开输出验证音色、力度、踏板和 Group 效果。

---

## 8. 已验证

- 真实 Live 12.3 工程模板 + 真实 MIDI：Group/子轨克隆、ID 重映射、Arrangement Clip 注入、CC64 Clip 自动化、Tempo 自动化，XML/gzip 校验通过。
- 单通道 1,228 音符、含 CC64 自动化：输出约 184 KB `.als`。
- 5.4 万音符黑乐谱、15 个通道组：输出约 5.0 MB `.als`（57.8 MB XML），最大复音 89，3 个 Tempo 自动化点。
- Group 效果器克隆测试：原型 Group 的 DrumBuss / MultibandDynamics / Eq8 被完整克隆；输出无重复全局 ID，所有 `PointeeId` 引用可解析。
- `cargo test`：7 个单元测试通过；另有 1 个手动 Group 集成测试（`--ignored`）。

---

## 9. 需要你确认

1. **模板**：请提供 `template.als`：一个 Group，命名 `A01`，里面恰好一条 Grand Piano MIDI 轨 + 一个占位 Clip。
2. **通道/乐器**：所有通道都用 Grand Piano，还是不同通道要映射不同 Live 乐器？如果要不同乐器，需要按 A01/A02 准备不同原型 Group。
3. **Group 效果差异**：每个通道的 Group 效果器/自动化是否相同？目前所有通道克隆同一个原型 Group。
4. **Pitch Bend / 其他非 CC 自动化**：是否需要写入 Pitch Bend、Channel Pressure、Poly Pressure？
5. **Program Change**：是否需要按 Program Change 切换音色/分轨？
6. **同通道多轨**：默认每个源 MIDI 轨道一条子轨；是否保持？
7. **Live 版本**：只自用 12.3，还是需要兼容 11/10？如果要兼容 10，请提供一个 Live 10 模板。
8. **测试样本**：能否给一个 10–30 秒 MIDI + 期望 BPM/参考音频，用于 A/B 验证时值、力度、踏板和 Group 效果？

---

## License

MIT OR Apache-2.0
