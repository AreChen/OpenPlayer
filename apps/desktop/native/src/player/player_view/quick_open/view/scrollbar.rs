use std::{cell::Cell, rc::Rc};

use gpui::{
    Bounds, Div, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, ScrollHandle, Stateful,
    UniformListScrollHandle, canvas, div, fill, point, prelude::*, px, size,
};

use crate::player::theme::Palette;

pub(super) fn quick_scrollbar(handle: &UniformListScrollHandle, palette: Palette) -> Stateful<Div> {
    let scroll = handle.0.borrow().base_handle.clone();
    let bounds = Rc::new(Cell::new(Bounds::default()));
    let measure = bounds.clone();
    let drag_bounds = bounds.clone();
    let drag_scroll = scroll.clone();
    let paint_scroll = scroll.clone();
    div()
        .id("quick-grid-scrollbar")
        .absolute()
        .top(px(14.))
        .bottom(px(12.))
        .right(px(2.))
        .w(px(12.))
        .child(
            canvas(
                move |rect, _, _| measure.set(rect),
                move |rect, _, window, _| {
                    let Some((thumb, travel, maximum)) = scrollbar_metrics(&paint_scroll, rect)
                    else {
                        return;
                    };
                    let ratio = f32::from(-paint_scroll.offset().y) / f32::from(maximum);
                    let track = rect.origin.x + px(4.);
                    let position = rect.origin.y + travel * ratio.clamp(0., 1.);
                    window.paint_quad(
                        fill(
                            Bounds::new(
                                point(track, rect.origin.y),
                                size(px(4.), rect.size.height),
                            ),
                            palette.line,
                        )
                        .corner_radii(px(2.)),
                    );
                    window.paint_quad(
                        fill(
                            Bounds::new(point(track, position), size(px(4.), thumb)),
                            palette.faint,
                        )
                        .corner_radii(px(2.)),
                    );
                },
            )
            .size_full(),
        )
        .on_mouse_down(
            MouseButton::Left,
            move |event: &MouseDownEvent, window, cx| {
                if scroll_to_point(&scroll, bounds.get(), event.position.y) {
                    window.refresh();
                    cx.stop_propagation();
                }
            },
        )
        .on_mouse_move(move |event: &MouseMoveEvent, window, _| {
            if event.pressed_button == Some(MouseButton::Left)
                && scroll_to_point(&drag_scroll, drag_bounds.get(), event.position.y)
            {
                window.refresh();
            }
        })
}

fn scrollbar_metrics(
    scroll: &ScrollHandle,
    track: Bounds<Pixels>,
) -> Option<(Pixels, Pixels, Pixels)> {
    let maximum = scroll.max_offset().y;
    let viewport = scroll.bounds().size.height;
    if maximum <= px(0.) || viewport <= px(0.) || track.size.height <= px(24.) {
        return None;
    }
    let fraction = f32::from(viewport) / f32::from(viewport + maximum);
    let thumb = (track.size.height * fraction).max(px(24.));
    Some((thumb, track.size.height - thumb, maximum))
}

fn scroll_to_point(scroll: &ScrollHandle, track: Bounds<Pixels>, y: Pixels) -> bool {
    let Some((thumb, travel, maximum)) = scrollbar_metrics(scroll, track) else {
        return false;
    };
    if travel <= px(0.) {
        return false;
    }
    let ratio = f32::from(y - track.origin.y - thumb * 0.5) / f32::from(travel);
    scroll.set_offset(point(scroll.offset().x, -maximum * ratio.clamp(0., 1.)));
    true
}
