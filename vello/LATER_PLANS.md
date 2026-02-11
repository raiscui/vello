# LATER_PLANS

## 2026-02-11 12:44:01: inner_shadow 示例后续增强(未落地)

- [ ] HiDPI/scale_factor: 把示例参数从"物理 px"改为"逻辑 px(CSS px)",并在渲染时统一乘 scale_factor,避免 Retina 下尺寸与手感偏差.
- [ ] SampleMode: 增加按键切换(自适应大面板/按钮Md/同时显示),减少两个样本同时出现时的干扰.
- [ ] Button presets: 追加 ButtonSize::Sm/Lg 等更多固定尺寸,用于不同控件的内阴影对照调参.

## 2026-02-11 15:13:14: 用户确认 vello example 不做 HiDPI 处理

- 结论: `examples/inner_shadow` 保持"按物理 px 绘制"即可.
- 原因:
  - 该示例只是调参对照与渲染实验.
  - 上层框架 `iced_emg` 已经在实际产品链路中处理了 HiDPI/scale_factor.
- 影响:
  - `vello/vello/LATER_PLANS.md` 里之前记录的 "HiDPI/scale_factor" 作为备忘保留,但后续不再推进落地.
