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

### 2026-02-10 23:57:47

- 我正在把 `origin/main` 合并到本地 `main`.
- 我这样做的原因是:
  - 当前 `main` 相对 `origin/main` 处于 `ahead 2, behind 44` 状态.
  - 需要把上游 44 个提交合并进来,同时保留本地 2 个提交.
- 我采用的方案是(已决定):
  - 使用 `git merge origin/main` 的合并策略,保留 merge commit,不做 rebase.
  - 合并前创建一个本地备份分支,确保随时可回滚.
- 我接下来要做的是:
  - `git fetch origin` 更新远端引用.
  - 创建备份分支 `backup/main-before-merge-20260210-235747`.
  - 执行 merge,如有冲突逐个解决并完成提交.
  - 合并后做最小验证,优先跑与本地改动直接相关的测试.

### 2026-02-11 00:01:39

- 我已经完成了 `origin/main` -> `main` 的合并.
- 我实际执行的动作是:
  - `git fetch origin`
  - `git branch backup/main-before-merge-20260210-235747`
  - `git merge --no-edit origin/main`
- 合并结果:
  - 当前 merge commit 为 `76f5771f`
  - `main` 相对 `origin/main` 为 `ahead 3, behind 0`
- 回归验证:
  - `cargo test -p inner_shadow` ✅

### 2026-02-11 12:38:12

- 我正在给 `examples/inner_shadow` 增加一个"按钮 Md(108x36,r=8)"的固定样本.
- 我这样做的原因是:
  - 当前示例的主形状会随窗口自适应,不便于直接对齐实际按钮尺寸.
  - 你给了 `ButtonSize::Md` 的具体像素,我需要在示例里做 1:1 对照,才能更快调出"按钮最佳内阴影"参数.

- 方案A(不惜代价,最佳方案):
  - 增加 `SampleMode`,支持在"自适应大面板/按钮Md/同时显示"之间切换.
  - 并把当前 mode 显示在窗口标题里,避免调参时忘记自己在看哪个样本.

- 方案B(先能用,后面再优雅)(本次采用):
  - 保留现有自适应大面板.
  - 额外绘制一个 108x36 的按钮样本(同一套 shadow 参数),用于实时对照调参.
  - 按窗口空间自动把按钮样本放在大面板的上方或下方,尽量避免重叠.

- 我接下来要做的是:
  - 修改 `examples/inner_shadow/src/main.rs`,抽 helper 复用"填充/描边/内阴影"绘制逻辑.
  - 跑 `cargo fmt` + `cargo test -p inner_shadow` 做回归验证.

### 2026-02-11 12:44:01

- 我已经完成了"按钮 Md(108x36,r=8)"样本的加入.
- 我做的改动是:
  - `examples/inner_shadow/src/main.rs`:
    - 默认 `corner_radius` 调整为 8px(更贴近按钮 Md 默认圆角).
    - 新增 `compute_button_md_rounded_rect`,在场景里额外绘制一个按钮 Md 尺寸样本.
    - 抽出 `draw_inset_shadow_sample`,复用"填充/描边/内阴影"绘制逻辑.
- 回归验证:
  - `cargo fmt` ✅
  - `(cd vello && cargo test -p inner_shadow)` ✅

### 2026-02-11 12:51:03

- 补充: 我在 vello workspace 内执行了 `(cd vello && cargo fmt)`.
- 我这样做的原因是:
  - 之前我只在主仓库根目录跑过 `cargo fmt`,它不会覆盖 submodule(vello) 内的 crate.
  - 结果 rustfmt 对 `examples/inner_shadow/src/main.rs` 做了小幅折行调整,如果不提交会反复产生噪声 diff.
- 处置:
  - 我已把该格式化改动一并提交.

### 2026-02-11 12:58:08

- 我正在把 `examples/inner_shadow` 的默认参数改成你刚才在窗口标题里确认的那组值.
- 我这样做的原因是:
  - 你已经通过实际观感找到了"按钮最佳内阴影"的基准参数.
  - 把它设为默认值,能让后续调参/对照更快进入状态,也便于截图/复现.

- 这次要设为默认值的参数是:
  - offset-x: 0
  - offset-y: 4
  - blur-radius: 23
  - spread-radius: 2
  - opacity: 0.46
  - border-radius: 8

- 我接下来要做的是:
  - 修改 `examples/inner_shadow/src/main.rs` 里的 `InsetBoxShadowParams::default()`.
  - 运行 `(cd vello && cargo test -p inner_shadow)` 验证.

### 2026-02-11 12:58:51

- 我已经把你给的这组参数设成默认值.
- 改动位置:
  - `examples/inner_shadow/src/main.rs` -> `InsetBoxShadowParams::default()`.
- 回归验证:
  - `(cd vello && cargo fmt)` ✅
  - `(cd vello && cargo test -p inner_shadow)` ✅
