# BlackLive 开发规范

## 用来降AI味儿的规范

### UI开发规范

1. 禁止在标题/控件下，写用于操作说明/讲解的小标题/小字
2. 未经允许，所有控件严禁私自加边框，应该直接以控件颜色作为区分
3. 原则上不允许使用等宽字体，除非某个地方如果没有等宽字体就无法使用（比如这个地方是写代码的），需要经过特别豁免

### 其他规范

1. 我不喜欢阈值
2. 禁止使用缓存
3. 如果因为有更好的算法，fallback必须及时清理

## Harness使用规范

1. 能放在后台的shell或者子代理尽量放后台
2. 尽可能使用多个子代理以加快速度

## 提交规范

1. 提交信息必须以动词开头，例如 "fix"、"feat" 等。可以在后面加括号，代表哪里修改了。
2. 提交信息必须包含一个短的描述，例如 "修复 bug"、"添加新功能" 等。
3. 提交信息必须包含一个空行，然后是更详细的描述。
4. 每行前面有一个短-号，例如 "- 修复 bug"、"- 添加新功能" 等。
5. 标题也用中文写。
6. 修改内容旁可能会有相应的注释，功能如何描述以注释为优先。
7. 每次修改代码后，都进行一次commit。如果修改过程分多个独立步骤，则每一步都commit一次。如果有功能写坏了，可以随时git回滚。

<!-- OPENWIKI:START -->

## OpenWiki

This repository has a generated `openwiki/` evidence index. It is optional just-in-time context, not required startup reading.

- Do not enumerate, preload, or search wikis at task start. Use retrieval when the user asks for it, when unfamiliar architecture or dependency behavior materially affects the task, or when source inspection leaves an important uncertainty. Stop once the question is grounded.
- When those conditions apply and OpenWiki retrieval tools are available, use `openwiki_search` for just-in-time context and `openwiki_read` for the relevant complete sections. If search returns `workspace_required`, ask which listed workspace to use and retry with its ID.
- Use `openwiki_list_workspaces` or `openwiki_list_wikis` when workspace membership itself needs to be discovered.
- If the retrieval tools are unavailable, read `openwiki/quickstart.md` and follow its links to the relevant pages.
- Treat source code and tests as authoritative. A brief's unknowns and review items are verification gaps, not automatic requirements.
- Prefer the narrowest quiet validation that proves the changed behavior. Preserve complete failure output.

Use the repository's documented maintenance process to refresh generated pages after source changes. Check its configuration and completed runs before relying on automatic updates. Do not hand-edit generated OpenWiki pages unless explicitly asked; prefer updating source code/docs and letting OpenWiki regenerate.

<!-- OPENWIKI:END -->
