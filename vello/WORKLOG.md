# WORKLOG

## 2026-02-10 20:16:28: blur-radius 与阴影深度解耦,让 `X` 更像只调柔和度

### 现象

- 你反馈在示例中按 `X` 增加 blur-radius 时:
  - 边缘会更糯(这是对的).
  - 同时阴影"吃进去的范围"也会明显变大,手感像在调 spread(`V`).

### 根因

- 我之前的示例把 inner cutout 的 inset 写成了 `blur + spread`.
  - 这会导致 blur 除了改变过渡宽度,还会把 inner_rect 额外向内收缩.
  - 结果就是 `X` 也在推深度,看起来像 `V`.

### 修复

- `examples/inner_shadow/src/main.rs`
  - inner cutout 的 inset 改为只由 `spread` 决定.
  - blur-radius 只用于映射到 `std_dev(sigma)`,主要改变柔和度.

### 验证

- `cargo fmt` ✅
- `cargo test -p inner_shadow` ✅

## 2026-02-10 20:49:24: 修复 `DestOut` 扣洞不彻底(中心矩形残影)

### 现象

- 你反馈在示例中按 `V` 增加 spread-radius 时:
  - 阴影会按预期变厚.
  - 但中心会出现一块矩形残留,拐角也会显得更尖锐.

### 根因

- 我在 inner cutout 阶段使用了 `Compose::DestOut`,但绘制 cutout 时也复用了 `shadow_color(alpha=opacity)`.
- `DestOut` 的语义是: `dst = dst * (1 - src_alpha)`.
  - 当 `src_alpha < 1` 时,中心不可能被完全清空.
  - 所以会留下半透明的"中心残影",看起来就像中间多了一块矩形.

### 修复

- `examples/inner_shadow/src/main.rs`
  - inner cutout 改用 `alpha=1` 的不透明 `cutout_mask` 来扣洞.
  - `shadow_color(opacity)` 只用于 outer blur,负责阴影强度.

### 验证

- `cargo fmt` ✅
- `cargo test -p inner_shadow` ✅

## 2026-02-11 00:01:39: 合并 `origin/main` 到本地 `main`

### 背景

- 合并前分支状态: `main...origin/main [ahead 2, behind 44]`
- 目标: 同步上游 44 个提交,同时保留本地 2 个提交.

### 执行

- 更新远端引用:
  - `git fetch origin`
- 创建回滚点(备份分支):
  - `git branch backup/main-before-merge-20260210-235747`
- 执行合并:
  - `git merge --no-edit origin/main`

### 结果

- 合并提交: `76f5771f`
- 合并后分支状态: `main...origin/main [ahead 3, behind 0]`

### 验证

- `cargo test -p inner_shadow` ✅

## 2026-02-11 12:44:01: inner_shadow 增加按钮 Md(108x36,r=8)固定样本,用于调按钮内阴影

### 目标

- 你给了 `ButtonSize::Md` 的真实像素(108x36,r=8).
- 我需要把它落到 `examples/inner_shadow` 里.
- 这样你在按键调 offset/blur/spread/opacity 时,能直接看到按钮尺寸下的手感.

### 实施

- 修改 `examples/inner_shadow/src/main.rs`
  - 默认 `corner_radius` 从 28 调整为 8,更贴近按钮 Md 默认圆角.
  - 场景里同时绘制两份样本:
    1) 自适应大面板(原示例).
    2) 按钮 Md 固定尺寸样本(108x36).
  - 新增 `compute_button_md_rounded_rect` 计算按钮样本的位置.
    - 优先放在大面板上方.
    - 上方放不下就放下方.
    - 仍放不下就贴顶留 margin.
  - 抽出 `draw_inset_shadow_sample`,复用"填充/描边/内阴影"绘制逻辑.

### 验证

- `cargo fmt` ✅
- `(cd vello && cargo test -p inner_shadow)` ✅

## 2026-02-11 12:51:03: 补充: vello workspace rustfmt 折行调整已提交

- 我补跑了 `(cd vello && cargo fmt)`.
- rustfmt 对 `examples/inner_shadow/src/main.rs` 的函数调用做了小幅折行调整.
- 该变更已提交,避免后续反复出现格式化噪声 diff.

## 2026-02-11 12:58:51: inner_shadow 默认参数改为按钮内阴影基准值

### 需求

- 你在窗口标题里确认了当前最佳手感参数:
  - box-shadow: inset 0px 4px 23px 2px rgba(0,0,0,0.46);
  - border-radius: 8px
- 你希望把它作为示例默认值,方便后续调参/复现.

### 实施

- 修改 `examples/inner_shadow/src/main.rs` 的 `InsetBoxShadowParams::default()`:
  - offset-x: 0
  - offset-y: 4
  - blur-radius: 23
  - spread-radius: 2
  - opacity: 0.46
  - border-radius: 8

### 验证

- `(cd vello && cargo fmt)` ✅
- `(cd vello && cargo test -p inner_shadow)` ✅
