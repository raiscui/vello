// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! CSS `box-shadow: inset ...` 风格的内阴影示例.
//!
//! 目标:
//! - 只保留一条最小可用链路,用于当作"范例代码".
//! - 内阴影完全用 vello 现有的 `Scene::draw_blurred_rounded_rect_in` 组合出来.
//! - 参数语义尽量贴近 CSS:
//!   - offset-x/offset-y(px)
//!   - blur-radius(px)
//!   - spread-radius(px)
//!   - rgba() 的 alpha(这里用 opacity 直接控制)
//!
//! 组合方式(核心思路):
//! - 先画 outer_blur(模糊后的填充圆角矩形).
//! - 再用 `Compose::DestOut` 画 inner_blur,把中心扣掉,只留下边缘过渡带.
//!
//! 注意:
//! - 这条路线本质是"扣洞 ring"实现,理论上存在一条由 inner_cutout 决定的隐含边界.
//! - 但在 CSS 常见参数范围内,这条边界会被 blur 自然抹平,看起来更像浏览器 inset box-shadow.

use anyhow::Result;
use std::sync::Arc;
use vello::kurbo::{Affine, Rect, RoundedRect, Stroke, Vec2};
use vello::peniko::{BlendMode, Color, Compose, Fill, Mix};
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu;
use vello::{AaConfig, Renderer, RendererOptions, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::Window;

// -----------------------------------------------------------------------------
// 渲染生命周期状态.
// -----------------------------------------------------------------------------
#[derive(Debug)]
enum RenderState {
    Active {
        surface: Box<RenderSurface<'static>>,
        valid_surface: bool,
        window: Arc<Window>,
    },
    Suspended(Option<Arc<Window>>),
}

// -----------------------------------------------------------------------------
// CSS inset box-shadow 参数(示例版).
//
// 对齐意图:
// - 让窗口标题能直接输出一条可复制的 CSS 字符串,方便你做对照.
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
struct InsetBoxShadowParams {
    offset_x: f64,
    offset_y: f64,
    blur_radius: f64,
    spread_radius: f64,
    opacity: f32,
    corner_radius: f64,
}

impl Default for InsetBoxShadowParams {
    fn default() -> Self {
        Self {
            offset_x: 8.0,
            offset_y: 8.0,
            blur_radius: 12.0,
            spread_radius: 0.0,
            opacity: 0.35,
            // 按钮 Md 的默认圆角是 8px,这里用它做默认值,方便你直接调按钮内阴影手感.
            corner_radius: 8.0,
        }
    }
}

struct InsetShadowApp {
    context: RenderContext,
    renderers: Vec<Option<Renderer>>,
    state: RenderState,
    scene: Scene,
    params: InsetBoxShadowParams,
    modifiers: ModifiersState,
}

impl ApplicationHandler for InsetShadowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let cached_window = match &mut self.state {
            RenderState::Suspended(cached) => cached,
            _ => return,
        };

        // 1) 拿到窗口(优先复用挂起前缓存的窗口).
        let window = cached_window
            .take()
            .unwrap_or_else(|| create_winit_window(event_loop));

        // 2) 创建 surface.
        let size = window.inner_size();
        let surface_future = self.context.create_surface(
            window.clone(),
            size.width,
            size.height,
            wgpu::PresentMode::AutoVsync,
        );
        let surface = pollster::block_on(surface_future).expect("创建 surface 失败");

        // 3) 为该设备创建 renderer.
        self.renderers
            .resize_with(self.context.devices.len(), || None);
        self.renderers[surface.dev_id]
            .get_or_insert_with(|| create_vello_renderer(&self.context, &surface));

        // 4) 进入 Active 状态.
        update_window_title(&window, &self.params);
        window.request_redraw();
        self.state = RenderState::Active {
            surface: Box::new(surface),
            valid_surface: true,
            window,
        };
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        if let RenderState::Active { window, .. } = &self.state {
            self.state = RenderState::Suspended(Some(window.clone()));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let (surface, valid_surface, window) = match &mut self.state {
            RenderState::Active {
                surface,
                valid_surface,
                window,
            } if window.id() == window_id => (surface, valid_surface, window),
            _ => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::ModifiersChanged(m) => self.modifiers = m.state(),

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }

                // ---------------------------------------------------------
                // 键位设计(尽量贴近 CSS 语义):
                // - 方向键: offset-x/y
                // - Z/X:    blur-radius
                // - C/V:    spread-radius
                // - A/S:    opacity
                // - Q/W:    border-radius
                // - R:      reset
                // - Esc:    exit
                //
                // Shift: 加速步进.
                // ---------------------------------------------------------
                let fast = self.modifiers.shift_key();
                let step_xy = if fast { 8.0 } else { 1.0 };
                let step_blur = if fast { 4.0 } else { 1.0 };
                let step_spread = if fast { 4.0 } else { 1.0 };
                let step_opacity = if fast { 0.05 } else { 0.02 };
                let step_radius = if fast { 4.0 } else { 1.0 };

                let mut changed = false;
                match event.logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),

                    Key::Named(NamedKey::ArrowLeft) => {
                        self.params.offset_x -= step_xy;
                        changed = true;
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        self.params.offset_x += step_xy;
                        changed = true;
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        self.params.offset_y -= step_xy;
                        changed = true;
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        self.params.offset_y += step_xy;
                        changed = true;
                    }

                    Key::Character(ch) => {
                        let ch = ch.to_lowercase();
                        match ch.as_str() {
                            "z" => {
                                self.params.blur_radius =
                                    (self.params.blur_radius - step_blur).max(0.0);
                                changed = true;
                            }
                            "x" => {
                                self.params.blur_radius += step_blur;
                                changed = true;
                            }
                            "c" => {
                                self.params.spread_radius -= step_spread;
                                changed = true;
                            }
                            "v" => {
                                self.params.spread_radius += step_spread;
                                changed = true;
                            }
                            "a" => {
                                self.params.opacity =
                                    (self.params.opacity - step_opacity).clamp(0.0, 1.0);
                                changed = true;
                            }
                            "s" => {
                                self.params.opacity =
                                    (self.params.opacity + step_opacity).clamp(0.0, 1.0);
                                changed = true;
                            }
                            "q" => {
                                self.params.corner_radius =
                                    (self.params.corner_radius - step_radius).max(0.0);
                                changed = true;
                            }
                            "w" => {
                                self.params.corner_radius += step_radius;
                                changed = true;
                            }
                            "r" => {
                                self.params = InsetBoxShadowParams::default();
                                changed = true;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }

                if changed {
                    update_window_title(window, &self.params);
                    window.request_redraw();
                }
            }

            WindowEvent::Resized(size) => {
                if size.width != 0 && size.height != 0 {
                    self.context
                        .resize_surface(surface, size.width, size.height);
                    *valid_surface = true;
                    window.request_redraw();
                } else {
                    *valid_surface = false;
                }
            }

            WindowEvent::RedrawRequested => {
                if !*valid_surface {
                    return;
                }

                // 每帧重建 Scene.
                self.scene.reset();
                build_scene_inset_box_shadow(
                    &mut self.scene,
                    surface.config.width,
                    surface.config.height,
                    &self.params,
                );

                // 渲染到中间纹理,再 blit 到 surface.
                let width = surface.config.width;
                let height = surface.config.height;
                let device_handle = &self.context.devices[surface.dev_id];

                self.renderers[surface.dev_id]
                    .as_mut()
                    .unwrap()
                    .render_to_texture(
                        &device_handle.device,
                        &device_handle.queue,
                        &self.scene,
                        &surface.target_view,
                        &vello::RenderParams {
                            // 背景色: 这里用深灰,更容易观察 shadow 的边缘过渡.
                            base_color: Color::new([0.12, 0.12, 0.12, 1.0]),
                            width,
                            height,
                            antialiasing_method: AaConfig::Msaa16,
                        },
                    )
                    .expect("渲染到纹理失败");

                let surface_texture = surface
                    .surface
                    .get_current_texture()
                    .expect("获取 surface texture 失败");

                let mut encoder =
                    device_handle
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Surface Blit"),
                        });
                surface.blitter.copy(
                    &device_handle.device,
                    &mut encoder,
                    &surface.target_view,
                    &surface_texture
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default()),
                );
                device_handle.queue.submit([encoder.finish()]);
                surface_texture.present();

                device_handle.device.poll(wgpu::PollType::Poll).unwrap();
            }

            _ => {}
        }
    }
}

fn main() -> Result<()> {
    let mut app = InsetShadowApp {
        context: RenderContext::new(),
        renderers: vec![],
        state: RenderState::Suspended(None),
        scene: Scene::new(),
        params: InsetBoxShadowParams::default(),
        modifiers: ModifiersState::default(),
    };

    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut app).expect("运行 event loop 失败");
    Ok(())
}

// -----------------------------------------------------------------------------
// Window/Renderer 辅助函数.
// -----------------------------------------------------------------------------

fn create_winit_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    let attr = Window::default_attributes()
        .with_inner_size(LogicalSize::new(1044, 800))
        .with_resizable(true)
        .with_title("Vello Inset Box-Shadow");
    Arc::new(event_loop.create_window(attr).unwrap())
}

fn create_vello_renderer(render_cx: &RenderContext, surface: &RenderSurface<'_>) -> Renderer {
    Renderer::new(
        &render_cx.devices[surface.dev_id].device,
        RendererOptions::default(),
    )
    .expect("创建 renderer 失败")
}

fn update_window_title(window: &Window, params: &InsetBoxShadowParams) {
    // -------------------------------------------------------------
    // HUD 目标:
    // - 直接展示一条可复制的 CSS inset box-shadow 字符串.
    // - 让你能快速把同一组参数丢进浏览器做对照.
    // -------------------------------------------------------------
    let css = format!(
        "box-shadow: inset {:.1}px {:.1}px {:.1}px {:.1}px rgba(0,0,0,{:.2}); border-radius: {:.1}px;",
        params.offset_x,
        params.offset_y,
        params.blur_radius,
        params.spread_radius,
        params.opacity,
        params.corner_radius
    );
    let title =
        format!("Vello Inset Box-Shadow | {css} | Arrows/Z X/C V/A S/Q W/R (Shift=fast, Esc=quit)");
    window.set_title(&title);
}

// -----------------------------------------------------------------------------
// Scene 构建.
// -----------------------------------------------------------------------------

fn build_scene_inset_box_shadow(
    scene: &mut Scene,
    width: u32,
    height: u32,
    params: &InsetBoxShadowParams,
) {
    // -------------------------------------------------------------
    // 两个样本:
    // 1) 自适应大面板(原示例).
    // 2) 固定按钮 Md 尺寸(108x36,r=8),用于调按钮内阴影.
    // -------------------------------------------------------------
    let (panel_rect, panel_shape, panel_radius) =
        compute_centered_rounded_rect(width, height, params.corner_radius);
    let (button_rect, button_shape, button_radius) =
        compute_button_md_rounded_rect(width, height, panel_rect, params.corner_radius);

    // 面色/描边色保持一致,这样你能更直接对照不同尺寸下的阴影手感差异.
    let face_color = Color::new([0.00, 0.48, 1.00, 1.0]);
    let border_color = Color::new([0.35, 0.40, 0.48, 1.0]);

    // 1) 先画大面板.
    draw_inset_shadow_sample(
        scene,
        panel_rect,
        panel_shape,
        panel_radius,
        1.5,
        face_color,
        border_color,
        params,
    );

    // 2) 再画按钮 Md 样本(放在大面板上下方,尽量避免重叠).
    draw_inset_shadow_sample(
        scene,
        button_rect,
        button_shape,
        button_radius,
        1.0,
        face_color,
        border_color,
        params,
    );
}

fn draw_inset_shadow_sample(
    scene: &mut Scene,
    rect: Rect,
    shape: RoundedRect,
    radius: f64,
    border_width_px: f64,
    face_color: Color,
    border_color: Color,
    params: &InsetBoxShadowParams,
) {
    // 1) 画底色(按钮面).
    scene.fill(Fill::NonZero, Affine::IDENTITY, face_color, None, &shape);

    // 2) 描边,帮助观察边界.
    scene.stroke(
        &Stroke::new(border_width_px),
        Affine::IDENTITY,
        border_color,
        None,
        &shape,
    );

    // 3) inset box-shadow(内阴影).
    let shadow_color = Color::new([0.0, 0.0, 0.0, params.opacity]);
    draw_inset_box_shadow_rounded_rect(
        scene,
        rect,
        radius,
        shadow_color,
        Vec2::new(params.offset_x, params.offset_y),
        params.blur_radius,
        params.spread_radius,
    );
}

fn compute_centered_rounded_rect(
    width: u32,
    height: u32,
    corner_radius: f64,
) -> (Rect, RoundedRect, f64) {
    // ---------------------------------------------------------------------
    // 说明:
    // - 形状大小跟随窗口,但避免过小或过大.
    // - 位置/尺寸尽量做像素对齐,便于观察 blur 的对称性.
    // ---------------------------------------------------------------------
    let w = width as f64;
    let h = height as f64;

    let rect_w = (w * 0.58).clamp(240.0, 720.0).round();
    let rect_h = (h * 0.42).clamp(180.0, 520.0).round();

    let x0 = ((w - rect_w) * 0.5).round();
    let y0 = ((h - rect_h) * 0.5).round();
    let x1 = x0 + rect_w;
    let y1 = y0 + rect_h;

    let base_rect = Rect::new(x0, y0, x1, y1);
    let max_radius = 0.5 * base_rect.width().min(base_rect.height());
    let radius = corner_radius.clamp(0.0, max_radius);
    let base_shape = RoundedRect::new(x0, y0, x1, y1, radius);

    (base_rect, base_shape, radius)
}

fn compute_button_md_rounded_rect(
    width: u32,
    height: u32,
    panel_rect: Rect,
    corner_radius: f64,
) -> (Rect, RoundedRect, f64) {
    // ---------------------------------------------------------------------
    // ButtonSize::Md(来自主工程按钮规格):
    // - height_px: 36
    // - min_width_px: 108
    // - corner_radius_px: 8
    // - border_width_px: 1
    //
    // 说明:
    // - 这里的样本目标是"真实按钮尺寸",方便你调 inset shadow 的最佳参数.
    // - 布局策略: 尽量放在大面板上方,放不下就放下方,再不行就贴顶边留 margin.
    // ---------------------------------------------------------------------
    let w = width as f64;
    let h = height as f64;

    let button_w = 108.0;
    let button_h = 36.0;

    let gap = 32.0;
    let margin = 24.0;

    // 水平居中.
    let x0 = ((w - button_w) * 0.5).round();
    let x1 = (x0 + button_w).round();

    // 尝试放到大面板上方.
    let mut y0 = panel_rect.y0 - gap - button_h;
    if y0 < margin {
        // 上方放不下就放下方.
        y0 = panel_rect.y1 + gap;
        if y0 + button_h > h - margin {
            // 下方也放不下就退化为贴顶,保证窗口变小时仍能看到按钮样本.
            y0 = margin;
        }
    }
    let y0 = y0.round();
    let y1 = (y0 + button_h).round();

    let rect = Rect::new(x0, y0, x1, y1);
    let max_radius = 0.5 * rect.width().min(rect.height());
    let radius = corner_radius.clamp(0.0, max_radius);
    let shape = RoundedRect::new(rect.x0, rect.y0, rect.x1, rect.y1, radius);

    (rect, shape, radius)
}

// -----------------------------------------------------------------------------
// CSS 参数 -> vello 绘制参数的映射.
// -----------------------------------------------------------------------------

fn css_blur_radius_to_std_dev(blur_radius_px: f64) -> f64 {
    // -----------------------------------------------------------------
    // CSS blur-radius 是一个"直觉像素尺度",而 vello 的 `std_dev` 是高斯 sigma.
    //
    // 我们在示例里用一个简单且可解释的映射:
    // - vello 的 blur kernel 截断范围约为 2.5*sigma.
    // - 因此把 blur-radius 近似为这段范围,令 sigma = blur/2.5.
    // -----------------------------------------------------------------
    (blur_radius_px.max(0.0)) / 2.5
}

fn draw_inset_box_shadow_rounded_rect(
    scene: &mut Scene,
    rect: Rect,
    radius: f64,
    shadow_color: Color,
    offset: Vec2,
    blur_radius_px: f64,
    spread_radius_px: f64,
) {
    // -----------------------------------------------------------------
    // 组合公式(示例版):
    //
    //   shadow = blur(outer_rect) - blur(inner_rect)     (Compose::DestOut)
    //
    // 其中:
    // - outer_rect: base rect + padding(用于 offset/blur 覆盖).
    // - inner_rect: 用来"扣掉中心".
    //   - inset 主要由 spread 控制(深度/厚度).
    //   - blur 主要改变过渡带的柔和度(宽度),但不会再额外推动 inner_rect 向内收缩.
    // -----------------------------------------------------------------
    let min_edge = rect.width().min(rect.height());
    if min_edge <= 1.0 {
        return;
    }

    // 1) 基础 clamp(避免 radius 失控).
    let max_radius = 0.5 * min_edge;
    let radius = radius.clamp(0.0, max_radius);

    // 2) blur-radius(px) -> std_dev(sigma).
    let blur_radius_px = blur_radius_px.max(0.0);
    let std_dev = css_blur_radius_to_std_dev(blur_radius_px);

    // 3) spread-radius(px):
    // - CSS 允许负值.
    // - 这里做一个温和 clamp,避免 inner_rect 过度膨胀导致无意义的算力浪费.
    let max_spread_pos = 0.5 * min_edge;
    let max_spread_neg = min_edge;
    let spread_radius_px = spread_radius_px.clamp(-max_spread_neg, max_spread_pos);

    // 4) inner cutout 的 inset(决定"阴影吃进去多深").
    //
    // 你反馈的关键调参手感是:
    // - `blur-radius`(Z/X) 更像"只改变柔和度",不要像 spread 一样显著改变阴影深度.
    //
    // 因此这里让 inset **只由 spread 控制**:
    // - spread=0: inner_rect == base_rect,依然会得到"纯 blur"的 inset shadow.
    // - spread>0: inner_rect 向内收缩,阴影更深.
    // - spread<0: inner_rect 向外扩张,阴影更浅甚至消失(更贴近 CSS 负 spread 的直觉).
    let inner_inset_px = spread_radius_px;

    let base_shape = RoundedRect::from_rect(rect, radius);
    let offset_rect = |r: Rect| {
        Rect::new(
            r.x0 + offset.x,
            r.y0 + offset.y,
            r.x1 + offset.x,
            r.y1 + offset.y,
        )
    };

    // 5) outer padding:
    // - 用 `|offset|max + blur` 作为 padding,避免 offset 后缺口.
    let offset_extent = offset.x.abs().max(offset.y.abs()).max(0.0);
    let outer_pad = offset_extent + blur_radius_px;
    let outer_rect = rect.inflate(outer_pad, outer_pad);
    let outer_min_edge = outer_rect.width().min(outer_rect.height());
    let outer_max_radius = 0.5 * outer_min_edge;
    let outer_radius = (radius + outer_pad).clamp(0.0, outer_max_radius);

    // 6) inner cutout(用来扣掉中心区域).
    let mut inner_rect = rect.inflate(-inner_inset_px, -inner_inset_px);
    if inner_rect.width() <= 1.0 || inner_rect.height() <= 1.0 {
        // 退化时折叠成中心 1x1,避免传入无效宽高.
        let c = rect.center();
        inner_rect = Rect::new(c.x - 0.5, c.y - 0.5, c.x + 0.5, c.y + 0.5);
    }
    let inner_min_edge = inner_rect.width().min(inner_rect.height());
    let inner_max_radius = 0.5 * inner_min_edge;

    // ---------------------------------------------------------
    // 关键手感修正(对应你反馈的"V 一增大,中心变矩形且拐角锐利"):
    //
    // 如果我们用"几何 inset"的直觉公式:
    // - `inner_radius = radius - spread`
    //
    // 那当 spread 接近/超过 radius 时,inner_radius 会很快 clamp 到 0,
    // 于是 inner cutout 就会退化成"直角矩形",你会明显看到:
    // - 中心露出一个矩形洞,
    // - 阴影的内边界拐角很尖锐.
    //
    // 但你要的是更像 CSS 的调参手感:
    // - spread 主要控制"深度/厚度",
    // - 而拐角不要因为 spread 立刻变得更尖锐.
    //
    // 因此这里让 inner cutout 的圆角半径保持跟外轮廓一致(并按尺寸上限 clamp):
    // - 这样 V 主要改变 inner_rect 的位置(深度),不会把圆角半径直接扣到 0.
    //
    // 备注:
    // - 你给的 shadertoy "Rounded Box - exact" SDF 公式,本质也是在 corner 处提供更合理的距离度量.
    // - 我们这里没有直接引入 SDF depth mask,而是用更小的改动达到"拐角更圆润"的目标.
    // ---------------------------------------------------------
    let inner_radius = radius.clamp(0.0, inner_max_radius);

    // 7) 外层 layer: 合成方式等价于 CSS 的正常 alpha blending.
    let blend = BlendMode::new(Mix::Normal, Compose::SrcOver);
    scene.push_layer(Fill::NonZero, blend, 1.0, Affine::IDENTITY, &base_shape);

    // 7.1 outer blur(并限制计算区域在 base_shape 内).
    scene.draw_blurred_rounded_rect_in(
        &base_shape,
        Affine::IDENTITY,
        offset_rect(outer_rect),
        shadow_color,
        outer_radius,
        std_dev,
    );

    // 7.2 inner blur: 用 DestOut 扣洞,清空中心.
    //
    // 关键点:
    // - `Compose::DestOut` 只看 src 的 alpha.
    // - 如果我们用 `shadow_color`(alpha=opacity) 来扣洞,那么中心最多只能扣掉 opacity,
    //   会留下一个半透明的"中心矩形残影"(你反馈的那个现象).
    // - 因此这里用一个 alpha=1 的不透明 mask 来做 cutout,确保中心真正被清空.
    let cutout_mask = Color::new([0.0, 0.0, 0.0, 1.0]);
    scene.push_layer(
        Fill::NonZero,
        Compose::DestOut,
        1.0,
        Affine::IDENTITY,
        &base_shape,
    );
    scene.draw_blurred_rounded_rect_in(
        &base_shape,
        Affine::IDENTITY,
        offset_rect(inner_rect),
        cutout_mask,
        inner_radius,
        std_dev,
    );
    scene.pop_layer();

    scene.pop_layer();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_radius_zero_maps_to_zero_sigma() {
        assert_eq!(css_blur_radius_to_std_dev(0.0), 0.0);
    }

    #[test]
    fn blur_radius_maps_by_cutoff_ratio() {
        // 2.5*sigma ~= blur_radius
        assert!((css_blur_radius_to_std_dev(25.0) - 10.0).abs() < 1e-9);
    }
}
