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
