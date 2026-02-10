# 笔记: inner_shadow(CSS inset box-shadow)

## 记录(只追加)

### 2026-02-10 20:16:28

- 现象: blur-radius 增大时,阴影不仅变柔,还会明显变"更深/更厚".
- 结论: 不要把 blur 直接加到 inner cutout 的 inset 上.
  - blur 自身已经会扩大过渡带宽度.
  - inset 应主要由 spread 控制,这样 `X` 更像只调柔和度.
- 备注: 你给的 `sdRoundBox exact` 更适合做"基于 SDF 的 depth mask/距离映射".
  - 当前示例走的是 "blur rect + DestOut 扣洞",不直接用 SDF.

### 2026-02-10 20:49:24

- 现象: `V`(spread) 增大时,中心会残留一个矩形区域,并且拐角显得更尖锐.
- 关键原因: `Compose::DestOut` 只看 src alpha.
  - 如果 cutout 也用 `shadow_color(alpha=opacity)`,中心最多只能扣掉 `opacity`,
    就会留下半透明残影(看起来像一块矩形"脏"在中间).
- 修复策略: cutout 用 `alpha=1` 的不透明 mask 来扣洞,shadow 的 opacity 只用于 outer blur.
