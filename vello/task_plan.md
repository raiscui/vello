# 任务计划: inner_shadow(CSS inset box-shadow 手感修正)

## 目标

- `examples/inner_shadow` 里:
  - `X`(blur-radius) 主要只改变"柔和度",不要再像 `V`(spread) 一样明显推动阴影深度.
  - `C/V`(spread-radius) 主要控制"阴影吃进去多深/多厚".
- 保持示例代码最小:
  - 只依赖 `Scene::draw_blurred_rounded_rect_in` + layer compose(`Compose::DestOut`).

## 阶段

- [x] 阶段1: 计划和设置
- [x] 阶段2: 修改实现
- [x] 阶段3: 回归验证
- [x] 阶段4: 交付记录

## 关键决定

- [已决定]: inner cutout 的 inset 只由 `spread` 决定.
  - 理由: blur 本身就会让边缘过渡更宽,如果再把 blur 叠加到 inset,会让 `X` 看起来像 `V`.

## 过程日志(只追加)

### 2026-02-10 20:16:28

- 我正在修正 `X`(blur-radius) 的调参手感.
- 我这样做的原因是:
  - 你观察到按 `X` 除了更糯,还会明显扩大阴影范围,像 `V`(spread) 的效果.
  - 这不符合你要的 CSS 直觉: blur 主要控制柔和度,spread 主要控制深度.
- 我做的改动是:
  - `examples/inner_shadow/src/main.rs`:
    - 把 inner cutout 的 inset 从 `blur + spread` 改为 `spread`.
- 回归验证:
  - `cargo fmt` ✅
  - `cargo test -p inner_shadow` ✅

### 2026-02-10 20:49:24

- 我正在继续修正 `V`(spread-radius) 时出现的"中心矩形残留 + 拐角偏锐利"现象.
- 我这样做的原因是:
  - 你反馈现在按 `V` 会让阴影扩大,但中间会出现一个矩形区域,并且边角看起来比原模型更尖锐.
  - 这通常意味着我们在 `Compose::DestOut` 扣洞时,没有把中心完全清空(留下了半透明残影).
- 我接下来要做的是:
  - 把 inner cutout 绘制从"用 shadow_color(带 opacity)"改成"用全不透明 alpha=1 的 mask".
  - 这样 `DestOut` 才能把中心区域真正扣干净,避免出现矩形残留.
  - 修完后再观察 `V` 的拐角手感,如仍偏锐,再考虑把 `BlurRect` 距离函数替换为你给的 `sdRoundBox exact` 思路(更平滑的圆角距离度量).

- 我做的改动是:
  - `examples/inner_shadow/src/main.rs`:
    - `Compose::DestOut` 的 inner cutout 改用 `alpha=1` 的 `cutout_mask`,
      避免用 `shadow_color(opacity)` 导致中心扣不干净.
- 回归验证:
  - `cargo fmt` ✅
  - `cargo test -p inner_shadow` ✅
