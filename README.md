# Blacklive

把 MIDI 转换成 Ableton Live `.als` 工程，采用 **模板注入** 而不是从零生成 `.als`。

- 输入：标准 MIDI（`.mid` / `.midi`）
- 模板：用户在自己的 Ableton Live 版本里保存的干净工程（一条 MIDI 轨道 + Grand Piano + 一个占位 Clip）
- 输出：可以直接用 Live 打开、按空格播放的 `.als`
- GUI：egui / eframe **0.37**
- 目标规模：约 1 万音符；已验证 5.4 万音符的黑乐谱 MIDI

> 目前已经用一个真实的 Live 12.3 工厂模板 + 真实 MIDI 跑通了完整链路，并通过 XML/gzip 自检；尚未在 Live 里实际点开试听。最后一步需要你用自己的 `template.als` 验证 Grand Piano 轨道。

---

## 1. 为什么可行

`.als` 是 gzip 压缩的 XML。Ableton 的 MIDI Clip 音符结构在 Live 10/11/12 中基本稳定：

```xml
<MidiClip Id="0" Time="0">
  <CurrentStart Value="0" />
  <CurrentEnd Value="16" />
  <Loop>
    <LoopStart Value="0" />
    <LoopEnd Value="16" />
    <LoopOn Value="true" />
  </Loop>
  <Notes>
    <KeyTracks>
      <KeyTrack Id="0">
        <Notes>
          <MidiNoteEvent Time="0" Duration="0.5"
                         Velocity="100" OffVelocity="64" NoteId="1" />
        </Notes>
        <MidiKey Value="60" />
      </KeyTrack>
    </KeyTracks>
    <NoteIdGenerator><NextId Value="2" /></NoteIdGenerator>
  </Notes>
</MidiClip>
```

关键点：

- 音符不是直接放在 `MidiClip` 下，而是按音高分组：
  `Notes / KeyTracks / KeyTrack / Notes / MidiNoteEvent`。
- `Time` / `Duration` 的单位是 **beat（四分音符）**，不是秒，也不是固定的 96 PPQ。
  MIDI 的 `tick / ppq` 直接就是 beat；PPQ 可以是 96、240、480……
- 同一个 Clip 还有 `CurrentStart` / `CurrentEnd` / `LoopStart` / `LoopEnd` / `LoopOn`。
  只替换音符、不改这些字段，长曲会只播放模板那一小段，或者循环。
- Session Clip 不会在 Arrangement 里按空格自动播放。转换器会：
  - 优先直接修改模板里的 Arrangement Clip；
  - 如果模板只有 Session Clip，就把它克隆到同轨道的 `ArrangerAutomation/Events`，并把 Clip 设置成从第 1 小节开始。
- Tempo 变化可以写进 Live 的 Master Tempo 自动化（`MainTrack/AutomationEnvelopes`），不需要牺牲 MIDI 的变速信息。

与原方案相比，实际实现修正了几个点：

| 原方案 | 实际做法 |
|---|---|
| `roxmltree` 修改 XML | `roxmltree` 是只读的；改用可修改的 `xmltree` |
| 固定 96 PPQ | 使用 MIDI 文件实际 PPQ：`beat = tick / ppq` |
| 只替换 `MidiNoteEvent` | 同时更新 Clip 长度、Loop、CurrentStart/End、NoteIdGenerator |
| 只设置 BPM | 支持把 MIDI Tempo Map 写进 Live 的 Tempo 自动化 |
| Session Clip 即可 | 自动克隆到 Arrangement，保证打开后按空格能播放 |
| 同音高重叠随意配对 | 与 yinhe 一致，采用 LIFO；没有 Note Off 的音符丢弃，避免超长挂音 |
| 忽略采样率 | 采样率是 Live 的 Preferences 设置，不保存在 `.als` 里；BPM 才在工程里 |

---

## 2. 快速开始

### 构建

```bash
cd /Users/jieneng/Documents/GitHub/blacklive
cargo build --release
```

### GUI

```bash
cargo run --release --bin blacklive
```

GUI 支持：

- 选择 MIDI / 模板 `.als` / 输出 `.als`
- 拖拽 `.mid` / `.als` 到窗口
- 分析模板，列出所有 MIDI Clip，并选择要替换哪一个
- 解析 MIDI，显示音符数、通道、PPQ、BPM、最大复音、变速点、踏板事件
- 通道过滤、移调、BPM 覆盖、最短时值、尾部留白、起点、同音高重叠处理
- 转换后直接「在 Live 中打开」或「打开所在文件夹」

### CLI

```bash
# 查看模板结构、轨道、Clip、占位音符
cargo run --release --bin blacklive-cli -- inspect template.als

# 导出解压后的 XML，方便检查标签
cargo run --release --bin blacklive-cli -- inspect template.als --xml-out template.xml

# 查看 MIDI 诊断
cargo run --release --bin blacklive-cli -- midi input.mid

# 转换（默认替换 Arrangement 的第一个 Clip；没有则克隆 Session Clip）
cargo run --release --bin blacklive-cli -- convert input.mid template.als output.als

# 指定模板 Clip 序号（inspect 输出里的 #N）
cargo run --release --bin blacklive-cli -- convert input.mid template.als output.als --clip-index 0

# 指定模板 Clip 名称
cargo run --release --bin blacklive-cli -- convert input.mid template.als output.als --template-clip BLACKLIVE

# 只保留某些通道（0-based；留空=全部，默认排除 GM 鼓通道 index 9）
cargo run --release --bin blacklive-cli -- convert input.mid template.als output.als --channels 0-7

# 覆盖 BPM（会关闭 Tempo 自动化，使用固定 BPM）
cargo run --release --bin blacklive-cli -- convert input.mid template.als output.als --bpm 120
```

---

## 3. 制作 template.als

模板是整个方案的关键。模板里的一切（Live 版本、Grand Piano 设备、轨道结构、Master 设置）都会被原样保留。

1. 用**目标 Ableton Live 版本**新建一个空工程。
2. 删除不需要的轨道、Return 轨道和效果器。只保留：
   - 一条 MIDI 轨道
   - Master 上尽量干净
3. 在这条 MIDI 轨道上加载 **Live 自带的 Grand Piano**。
4. 在 **Arrangement 视图第 1 小节**放一个短 MIDI Clip（不是只在 Session 里）：
   - 长度 1 小节即可
   - 里面放 1 个短音符作为占位
   - 把 Clip 改名为 `BLACKLIVE`（方便识别；不强制）
5. BPM 随便设，转换时会根据 MIDI / 选项覆盖。
6. 保存为 `template.als`，然后关闭 Live。
7. 如果模板里同时有 Session Clip 和 Arrangement Clip，转换器默认修改 Arrangement Clip，并警告你 Session 里还有占位 Clip。

> 采样率（44.1k / 48k）是 Live 的全局 Preferences，不在 `.als` 里。请先在 Live 的 Audio 设置里设好。

### 检查模板 XML

```bash
gunzip -c template.als > template.xml
grep -n '<MidiClip' template.xml
grep -n '<MidiNoteEvent' template.xml | head
```

或者：

```bash
cargo run --release --bin blacklive-cli -- inspect template.als --xml-out template.xml
```

然后重点看：

- 根节点：`<Ableton MajorVersion="5" MinorVersion="12.0_..." Creator="Ableton Live ...">`
- 目标轨道：`<MidiTrack>` → `<Name>/<EffectiveName>`
- 设备：`<DeviceChain>/<DeviceChain>/<Devices>`
- Clip：`<MidiClip Time="0">`
- 音符：`<Notes>/<KeyTracks>/<KeyTrack>`
- 音符事件：`<MidiNoteEvent Time="..." Duration="..." Velocity="..." OffVelocity="..." NoteId="..." />`
- 音符 ID 发号器：`<NoteIdGenerator><NextId Value="N+1" /></NoteIdGenerator>`

Live 12.3 的真实节点顺序是：

```xml
<KeyTrack Id="0">
  <Notes>
    <MidiNoteEvent ... />
  </Notes>
  <MidiKey Value="60" />
</KeyTrack>
```

注意 `Notes` 在 `MidiKey` 前面。

---

## 4. 转换逻辑

### MIDI tick → beat

```text
beat = tick / ppq
```

Ableton 的 `Time` / `Duration` 直接使用这个 beat 值。96 PPQ 只是某些 MIDI 的选择，不是 Ableton 的固定单位。

### 同音高重叠配对

MIDI 本身不规定同一个 `(channel, key)` 上多个 Note On 时，哪个 Note Off 对应哪个 Note On。Blacklive 与 yinhe 的黑乐谱解析器保持一致，使用 **LIFO**：

> 最新的 Note On 先被 Note Off 结束。

这比 FIFO 更不容易产生超长音符。没有 Note Off 的音符会被丢弃并给出警告，避免生成几分钟的挂音。

### Tempo Map

- MIDI 的 tick 位置不受 tempo 影响，beat 位置可以直接保留。
- 如果 MIDI 有多个有效 Tempo 点，转换器会写进 Live 的 Master Tempo 自动化：
  - `MainTrack/AutomationEnvelopes/Envelopes/AutomationEnvelope`
  - `EnvelopeTarget/PointeeId` 指向 Tempo 的 `AutomationTarget Id`
  - `Automation/Events/FloatEvent Time=beat Value=BPM`
- 用同一 beat 上的两个事件（旧值 + 新值）表示瞬时变速，避免 Live 把变速画成渐变。
- 如果使用 `--bpm` 覆盖，则只写固定 BPM，并清掉模板原有的 Tempo 自动化。

### Clip 长度与播放

- `CurrentEnd = start + max(note end)`
- `LoopOn = false`
- `LoopStart = 0`，`LoopEnd = clip length`
- Arrangement Clip 的 `Time = start`（默认 0，即第 1 小节）
- 如果模板只有 Session Clip，会把它复制到 `ArrangerAutomation/Events`，并同时更新 Session 源 Clip。

### 输出安全

- 写入前重新解析生成的 XML，确保不会输出损坏的 XML。
- 先写同目录临时文件，再 `rename` 到目标 `.als`，避免中途失败覆盖已有工程。
- 保留模板的根版本、设备、轨道、Master 和未知节点；只改目标 Clip 和 Tempo。

---

## 5. 项目结构

```text
blacklive/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs              # 对外 API、数字格式化
│   ├── midi.rs             # MIDI 解析、LIFO 配对、Tempo Map、诊断
│   ├── als.rs              # gzip、XML、Clip 枚举、模板注入、Tempo 自动化
│   ├── convert.rs          # 过滤/移调/长度/BPM/转换编排
│   └── bin/
│       ├── blacklive-gui.rs   # eframe/egui 0.37 桌面界面
│       └── blacklive-cli.rs   # 命令行工具
└── target/                 # 构建产物（已 gitignore）
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

## 6. 当前限制

- **单轨 Grand Piano**：所有保留的 MIDI 音符都会进入一个 Clip / 一条轨道。
  多轨、按 channel / Program Change 分配 Live 乐器是下一步。
- **Program Change 被忽略**：不会根据音色切换轨道。
- **CC64 延音踏板被忽略**：目前只在诊断里提示。对钢琴音质有影响，后续可做「把踏板烘焙进音符时值」或写 CC 自动化。
- **Pitch Bend 被忽略**。
- **同一轨道的多个 Arrangement Clip 不会合并**：转换器只替换选中的 Clip，其他 Clip 原样保留。建议使用干净模板。
- **大文件**：当前用 `midly::Smf::parse` 一次性解析。1 万音符没问题；如果是 1.2 GB / 上亿音符，建议后续复用 yinhe 的 mmap + 惰性解析路径。
- **极短音符**：保留 MIDI tick 精度（至少 1 tick）。Live 钢琴卷帘里可能显示成细线，但 XML 时值是正确的；需要更明显可用 `--min-note` 拉长。
- **没有在 Live 里自动试听**：输出后需要你打开 Live 验证 Grand Piano 的实际声音、力度和时值。

---

## 7. 和 yinhe 的关系

同目录的 `yinhe` 项目是一个黑乐谱 MIDI 编辑器，和本项目解决的问题不同：

- yinhe 有成熟的 black-MIDI 解析、LIFO 同音高配对、音符桶和 egui 0.37 工程结构；
- 但 yinhe 目前没有 `.als` / Ableton 导出。

Blacklive 因此保持独立小工程，没有依赖 yinhe 的整个 workspace；只借鉴了：

- 同音高 Note On/Off 的 LIFO 配对；
- 对没有 Note Off 的音符采取丢弃而不是拉到文件末尾；
- eframe 0.37 的 `App::ui` 结构、系统字体/中文显示方式。

如果以后要把转换器做成 yinhe 的「导出为 Ableton 工程」功能，可以直接复用 yinhe 的解析器和文档模型，而不是现在的轻量 MIDI 解析器。

---

## 8. 已验证 / 待确认

已用真实的 Live 12.3 工厂模板和真实 MIDI 验证：

- 约 2 分钟、1,228 音符：输出 2.0 MB XML / 172 KB `.als`，XML/ gzip 校验通过；
- 5.4 万音符黑乐谱：输出 13 MB XML / 760 KB `.als`，最大复音 89，3 个 Tempo 自动化点，XML/gzip 校验通过；
- `cargo test` 7 个单元测试通过。

还需要你确认：

1. **目标 Live 版本**：目前机器上是 Live 12.3 Suite。是否只针对 12.3？
2. **模板**：能否提供你的 `template.als`（一条 MIDI 轨 + Grand Piano + 一个 Arrangement 占位 Clip）？
3. **音色范围**：源 MIDI 是否全部当钢琴合并？还是需要按通道/Program 分到几条 Live 乐器轨道？
4. **踏板**：是否需要把 CC64 烘焙成音符长度？这是钢琴听感最大的剩余差异。
5. **Tempo**：是否接受当前的 Tempo 自动化策略？特别是 tick 0 有多个 Tempo 事件的怪文件。
6. **同音高重叠**：默认保留；是否要默认截短前音 / 去重？
7. **输出位置**：默认从第 1 小节开始、Loop Off、Clip 名 = MIDI 文件名、轨道名保持模板名。

---

## License

MIT OR Apache-2.0
