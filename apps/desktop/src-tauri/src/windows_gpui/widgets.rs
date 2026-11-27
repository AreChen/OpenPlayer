use std::{cell::Cell, rc::Rc};

use gpui::{
    Bounds, Context, Div, ElementId, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Render, Rgba, SharedString, Stateful, Window, canvas, div, prelude::*,
    px, relative, transparent_black,
};
use serde_json::Value;

use super::{
    player_view::PlayerView,
    theme::{Palette, icon},
};

pub(super) fn item_id(category: &str, id: &str) -> SharedString {
    format!("{category}:{id}").into()
}

pub(super) fn tooltip(
    element: Stateful<Div>,
    label: impl Into<SharedString>,
    palette: Palette,
) -> Stateful<Div> {
    let label = label.into();
    element.tooltip(move |_, cx| {
        cx.new(|_| Tooltip {
            label: label.clone(),
            palette,
        })
        .into()
    })
}

struct Tooltip {
    label: SharedString,
    palette: Palette,
}

impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .max_w(px(470.))
            .rounded(px(7.))
            .bg(Rgba {
                a: 1.,
                ..self.palette.surface
            })
            .border_1()
            .border_color(self.palette.line)
            .shadow_md()
            .px_2()
            .py_1()
            .text_color(self.palette.text)
            .text_size(px(12.))
            .child(self.label.clone())
    }
}

pub(super) fn command(
    el: Stateful<Div>,
    kind: &'static str,
    value: Value,
    enabled: bool,
    cx: &Context<PlayerView>,
) -> Stateful<Div> {
    if enabled {
        el.cursor_pointer()
            .on_click(cx.listener(move |this, _, window, cx| {
                this.send_in_window(kind, value.clone(), window, cx);
            }))
    } else {
        el.opacity(0.42)
    }
}

pub(super) fn icon_button(
    id: impl Into<ElementId>,
    name: impl Into<SharedString>,
    palette: Palette,
    active: bool,
) -> Stateful<Div> {
    icon_button_box(id, palette, active).child(icon(name, palette.accent, 17.))
}

pub(super) fn icon_button_box(
    id: impl Into<ElementId>,
    palette: Palette,
    active: bool,
) -> Stateful<Div> {
    button_feedback(
        div()
            .id(id)
            .flex_none()
            .w(px(36.))
            .h(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(9.))
            .border_1()
            .border_color(transparent_black())
            .when(active, |el| el.bg(palette.accent_muted)),
        palette,
    )
}

pub(super) fn button_feedback(el: Stateful<Div>, palette: Palette) -> Stateful<Div> {
    el.hover(move |style| {
        style
            .bg(Rgba {
                a: 0.26,
                ..palette.accent
            })
            .border_color(palette.accent_border)
    })
    .active(move |style| {
        style
            .bg(Rgba {
                a: 0.38,
                ..palette.accent
            })
            .border_color(palette.accent)
    })
}

pub(super) fn text_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    palette: Palette,
    active: bool,
) -> Stateful<Div> {
    button_feedback(
        div()
            .id(id)
            .flex_none()
            .min_h(px(30.))
            .px_3()
            .py_1()
            .rounded(px(9.))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(13.))
            .text_color(if active { palette.accent } else { palette.text })
            .bg(if active {
                palette.accent_soft
            } else {
                palette.control
            })
            .border_1()
            .border_color(if active {
                palette.accent_border
            } else {
                palette.line
            })
            .child(label.into()),
        palette,
    )
}

pub(super) fn row(id: impl Into<ElementId>, palette: Palette, active: bool) -> Stateful<Div> {
    button_feedback(
        div()
            .id(id)
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .min_h(px(32.))
            .px_3()
            .py_1()
            .rounded(px(9.))
            .when(active, |el| {
                el.bg(palette.accent_soft).text_color(palette.accent)
            }),
        palette,
    )
}

pub(super) fn slider(
    kind: &'static str,
    value: f64,
    max: f64,
    palette: Palette,
    cx: &Context<PlayerView>,
) -> Stateful<Div> {
    let ratio = if max > 0. {
        (value / max).clamp(0., 1.) as f32
    } else {
        0.
    };
    let bounds: Rc<Cell<Bounds<Pixels>>> = Rc::default();
    let measured = bounds.clone();
    let el = div()
        .id(kind)
        .relative()
        .min_w(px(20.))
        .h(px(18.))
        .flex_1()
        .flex()
        .items_center()
        .child(
            div()
                .relative()
                .w_full()
                .h(px(4.))
                .rounded_full()
                .bg(palette.line)
                .child(
                    div()
                        .w(relative(ratio))
                        .h_full()
                        .rounded_full()
                        .bg(palette.accent),
                )
                .when(max > 0., |el| {
                    el.child(
                        div()
                            .absolute()
                            .top(px(-3.))
                            .left(relative(ratio))
                            .ml(px(-5.))
                            .size(px(10.))
                            .rounded_full()
                            .bg(palette.accent),
                    )
                }),
        )
        .child(
            canvas(move |rect, _, _| measured.set(rect), |_, _, _, _| {})
                .absolute()
                .inset_0()
                .size_full(),
        );
    if max <= 0. {
        return el;
    }
    if kind == "seek" {
        let drag_bounds = bounds.clone();
        let commit_bounds = bounds.clone();
        let release = cx.listener(move |this, event: &MouseUpEvent, _, cx| {
            if this.seek_preview.take().is_some() {
                this.send(
                    "seek",
                    Value::from(slider_value(event.position.x, commit_bounds.get(), max)),
                    cx,
                );
            }
        });
        return el
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.seek_preview = Some(slider_value(event.position.x, bounds.get(), max));
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                if this.seek_preview.is_some() && event.pressed_button == Some(MouseButton::Left) {
                    this.seek_preview =
                        Some(slider_value(event.position.x, drag_bounds.get(), max));
                    cx.notify();
                }
            }))
            .on_mouse_up(MouseButton::Left, release)
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if let Some(position) = this.seek_preview.take() {
                        this.send("seek", Value::from(position), cx);
                    }
                }),
            );
    }
    {
        let drag_bounds = bounds.clone();
        el.cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.send(
                        kind,
                        Value::from(slider_value(event.position.x, bounds.get(), max)),
                        cx,
                    );
                }),
            )
            .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                if event.pressed_button == Some(MouseButton::Left)
                    && drag_bounds.get().contains(&event.position)
                {
                    this.send(
                        kind,
                        Value::from(slider_value(event.position.x, drag_bounds.get(), max)),
                        cx,
                    );
                }
            }))
    }
}

fn slider_value(x: Pixels, bounds: Bounds<Pixels>, max: f64) -> f64 {
    let width = f64::from(bounds.size.width);
    if width > 0. {
        (f64::from(x - bounds.origin.x) / width).clamp(0., 1.) * max
    } else {
        0.
    }
}

pub(super) fn heading(label: impl Into<SharedString>, palette: Palette) -> impl IntoElement {
    div()
        .text_size(px(15.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(palette.text)
        .child(label.into())
}
