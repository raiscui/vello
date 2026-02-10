# ERRORFIX

## 2026-02-10 20:16:28: blur-radius 误参与 inset 深度(导致 `X` 手感像 `V`)

### 问题

- 在 `examples/inner_shadow` 里,按 `X` 增加 blur-radius 时,
  阴影不仅变柔,还会显著变深/变厚.

### 原因

- inner cutout 的 inset 误写为 `blur + spread`,
  导致 blur 同时在推进 inner_rect 的收缩,从而推进深度.

### 修复

- inset 改为只由 `spread` 决定.

### 验证

- `cargo fmt` ✅
- `cargo test -p inner_shadow` ✅

## 2026-02-10 20:49:24: `DestOut` 使用 opacity 导致中心扣不干净(出现矩形残影)

### 问题

- 在 `examples/inner_shadow` 里,按 `V` 增加 spread 时,
  中间会残留一块矩形区域,看起来像阴影没有被完全扣掉.

### 原因

- inner cutout 使用 `Compose::DestOut`,但我复用了 `shadow_color(alpha=opacity)` 来画 cutout.
- `DestOut` 只看 src alpha,当 `alpha < 1` 时,中心区域最多只能扣掉一部分,必然留下半透明残影.

### 修复

- cutout 改用 `alpha=1` 的不透明 mask(`cutout_mask`) 来扣洞.

### 验证

- `cargo fmt` ✅
- `cargo test -p inner_shadow` ✅
