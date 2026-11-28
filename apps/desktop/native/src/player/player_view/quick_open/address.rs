use std::{cell::RefCell, ops::Range};

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, Div, ElementInputHandler, Entity,
    EntityInputHandler, FocusHandle, KeyDownEvent, MouseButton, Pixels, Point, ShapedLine,
    SharedString, TextAlign, TextRun, UTF16Selection, Window, canvas, div, fill, point, prelude::*,
    px, size,
};

use crate::player::theme::Palette;

pub(super) struct AddressInput {
    text: String,
    anchor: usize,
    caret: usize,
    marked: Option<Range<usize>>,
    focus: FocusHandle,
    geometry: RefCell<Option<TextGeometry>>,
    dragging: bool,
}

impl AddressInput {
    pub(super) fn new(text: String, cx: &mut Context<Self>) -> Self {
        let caret = text.len();
        Self {
            text,
            anchor: 0,
            caret,
            marked: None,
            focus: cx.focus_handle(),
            geometry: RefCell::new(None),
            dragging: false,
        }
    }

    pub(super) fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus.focus(window, cx);
        self.move_selection(0, self.text.len(), window, cx);
    }

    pub(super) fn text(&self) -> &str {
        &self.text
    }

    pub(super) fn unmarked_text(&self) -> Option<&str> {
        self.marked.is_none().then_some(&self.text)
    }

    pub(super) fn is_focused(&self, window: &Window) -> bool {
        self.focus.is_focused(window)
    }

    fn selected(&self) -> Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }

    fn move_selection(
        &mut self,
        anchor: usize,
        caret: usize,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        self.anchor = anchor;
        self.caret = caret;
        window.notify_text_selection_changed(&self.focus);
        cx.notify();
    }

    fn replacement(&self, range: Option<Range<usize>>) -> Range<usize> {
        range
            .map(|range| byte_range(&self.text, range))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.selected())
    }

    fn replace(
        &mut self,
        range: Range<usize>,
        text: &str,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let text = text.lines().next().unwrap_or_default();
        let maximum = 32_767usize
            .saturating_sub(self.text.chars().count() - self.text[range.clone()].chars().count());
        let text: String = text
            .chars()
            .filter(|character| !character.is_control())
            .take(maximum)
            .collect();
        let caret = range.start + text.len();
        self.text.replace_range(range, &text);
        self.marked = None;
        self.anchor = caret;
        self.caret = caret;
        window.notify_text_input_changed(&self.focus);
        cx.notify();
    }

    fn index_at(&self, position: Point<Pixels>) -> Option<usize> {
        let geometry = self.geometry.borrow();
        let geometry = geometry.as_ref()?;
        let index = geometry
            .line
            .closest_index_for_x(position.x - geometry.origin.x);
        Some(self.text.floor_char_boundary(index.min(self.text.len())))
    }

    fn neighboring_index(&self, left: bool, word: bool) -> usize {
        if left {
            let mut chars = self.text[..self.caret].char_indices().rev();
            let Some((index, _)) = chars.next() else {
                return 0;
            };
            if word {
                chars
                    .find(|&(_, character)| is_path_separator(character))
                    .map_or(0, |(index, character)| index + character.len_utf8())
            } else {
                index
            }
        } else {
            let mut chars = self.text[self.caret..].char_indices();
            if word {
                chars
                    .find(|&(_, character)| is_path_separator(character))
                    .map_or(self.text.len(), |(index, character)| {
                        self.caret + index + character.len_utf8()
                    })
            } else {
                chars
                    .nth(1)
                    .map_or(self.text.len(), |(index, _)| self.caret + index)
            }
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if self.marked.is_some() {
            if matches!(key, "enter" | "escape" | "backspace") {
                cx.stop_propagation();
            }
            return;
        }
        if modifiers.alt || modifiers.platform {
            return;
        }
        let selected = self.selected();
        match (key, modifiers.control) {
            ("a", true) => self.move_selection(0, self.text.len(), window, cx),
            ("c" | "x", true) if !selected.is_empty() => {
                cx.write_to_clipboard(ClipboardItem::new_string(
                    self.text[selected.clone()].into(),
                ));
                if key == "x" {
                    self.replace(selected, "", window, cx);
                }
            }
            ("c" | "x", true) => {}
            ("v", true) => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.replace(selected, &text, window, cx);
                }
            }
            ("home" | "end" | "left" | "right", word) => {
                let caret = match key {
                    "home" => 0,
                    "end" => self.text.len(),
                    "left" if !modifiers.shift && !word && !selected.is_empty() => selected.start,
                    "right" if !modifiers.shift && !word && !selected.is_empty() => selected.end,
                    _ => self.neighboring_index(key == "left", word),
                };
                let anchor = if modifiers.shift { self.anchor } else { caret };
                self.move_selection(anchor, caret, window, cx);
            }
            ("backspace" | "delete", word) => {
                let range = if !selected.is_empty() {
                    selected
                } else if key == "backspace" {
                    self.neighboring_index(true, word)..self.caret
                } else {
                    self.caret..self.neighboring_index(false, word)
                };
                self.replace(range, "", window, cx);
            }
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl EntityInputHandler for AddressInput {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = byte_range(&self.text, range);
        *adjusted_range = Some(utf16_range(&self.text, range.clone()));
        Some(self.text[range].to_owned())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: utf16_range(&self.text, self.selected()),
            reversed: self.caret < self.anchor,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        Some(utf16_range(&self.text, self.marked.clone()?))
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace(self.replacement(range), text, window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        new_selection: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = self.replacement(range);
        let start = range.start;
        self.replace(range, text, window, cx);
        if start < self.caret {
            let marked = start..self.caret;
            if let Some(selection) = new_selection {
                let selection = byte_range(&self.text[marked.clone()], selection);
                self.anchor = start + selection.start;
                self.caret = start + selection.end;
            }
            self.marked = Some(marked);
        }
    }

    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let geometry = self.geometry.borrow();
        Some(
            geometry
                .as_ref()?
                .range_bounds(byte_range(&self.text, range)),
        )
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.text[..self.index_at(point)?].encode_utf16().count())
    }

    fn set_selected_text_range(
        &mut self,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = byte_range(&self.text, range);
        self.move_selection(range.start, range.end, window, cx);
    }

    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.text.encode_utf16().count())
    }
}

#[derive(Clone)]
struct TextGeometry {
    line: ShapedLine,
    origin: Point<Pixels>,
    viewport: Bounds<Pixels>,
}

impl TextGeometry {
    fn range_bounds(&self, range: Range<usize>) -> Bounds<Pixels> {
        let left = self.viewport.origin.x;
        let right = left + self.viewport.size.width;
        let start = (self.origin.x + self.line.x_for_index(range.start)).clamp(left, right);
        let end = (self.origin.x + self.line.x_for_index(range.end)).clamp(left, right);
        Bounds::new(
            point(start.min(end), self.origin.y),
            size((end - start).abs(), px(20.)),
        )
    }
}

pub(super) fn address_field(input: &Entity<AddressInput>, palette: Palette, cx: &App) -> Div {
    input_field(input, None, palette, cx)
}

pub(super) fn input_field(
    input: &Entity<AddressInput>,
    placeholder: Option<SharedString>,
    palette: Palette,
    cx: &App,
) -> Div {
    let state = input.read(cx);
    let text = state.text.clone();
    let placeholder = placeholder.filter(|_| text.is_empty());
    let selected = state.selected();
    let caret = state.caret;
    let marked = state.marked.clone();
    let focus = state.focus.clone();
    let measured = input.clone();
    let painted = input.clone();
    let key_input = input.clone();
    let pointer_input = input.clone();
    let motion_input = input.clone();
    let release_input = input.clone();
    let text_element = canvas(
        move |bounds, window, cx| {
            let run = TextRun {
                len: text.len(),
                font: window.text_style().font(),
                color: palette.text.into(),
                ..TextRun::default()
            };
            let line = window
                .text_system()
                .shape_line(text.into(), px(14.), &[run], None);
            let previous_scroll = measured
                .read(cx)
                .geometry
                .borrow()
                .as_ref()
                .map_or(px(0.), |previous| {
                    previous.viewport.origin.x - previous.origin.x
                });
            let caret_x = line.x_for_index(caret);
            let visible_width = (bounds.size.width - px(2.)).max(px(0.));
            let scroll = previous_scroll
                .min(caret_x)
                .max(caret_x - visible_width)
                .max(px(0.));
            let geometry = TextGeometry {
                line,
                viewport: bounds,
                origin: point(
                    bounds.origin.x - scroll,
                    bounds.origin.y + (bounds.size.height - px(20.)) / 2.,
                ),
            };
            measured.read(cx).geometry.replace(Some(geometry.clone()));
            geometry
        },
        move |bounds, geometry, window, cx| {
            let active = focus.is_focused(window);
            if active {
                window.handle_input(&focus, ElementInputHandler::new(bounds, painted), cx);
                if !selected.is_empty() {
                    window.paint_quad(fill(geometry.range_bounds(selected), palette.accent_muted));
                }
            }
            let _ =
                geometry
                    .line
                    .paint(geometry.origin, px(20.), TextAlign::Left, None, window, cx);
            if active {
                if let Some(marked) = marked {
                    let bounds = geometry.range_bounds(marked);
                    let underline = Bounds::new(
                        point(bounds.origin.x, bounds.origin.y + px(18.)),
                        size(bounds.size.width, px(1.)),
                    );
                    window.paint_quad(fill(underline, palette.accent));
                }
                let caret = geometry.range_bounds(caret..caret).origin;
                window.paint_quad(fill(
                    Bounds::new(caret, size(px(1.5), px(20.))),
                    palette.accent,
                ));
            }
        },
    )
    .size_full();

    div()
        .relative()
        .flex_1()
        .min_w_0()
        .h_full()
        .px_2()
        .overflow_hidden()
        .cursor(CursorStyle::IBeam)
        .track_focus(&input.read(cx).focus)
        .on_key_down(move |event, window, cx| {
            key_input.update(cx, |input, cx| input.key_down(event, window, cx));
        })
        .on_mouse_down(MouseButton::Left, move |event, window, cx| {
            pointer_input.update(cx, |input, cx| {
                input.focus.focus(window, cx);
                let Some(caret) = input.index_at(event.position) else {
                    return;
                };
                input.dragging = true;
                if event.click_count > 1 {
                    input.move_selection(0, input.text.len(), window, cx);
                } else {
                    let anchor = if event.modifiers.shift {
                        input.anchor
                    } else {
                        caret
                    };
                    input.move_selection(anchor, caret, window, cx);
                }
            });
            cx.stop_propagation();
        })
        .on_mouse_move(move |event, window, cx| {
            if event.dragging() {
                motion_input.update(cx, |input, cx| {
                    if input.dragging
                        && let Some(caret) = input.index_at(event.position)
                    {
                        input.move_selection(input.anchor, caret, window, cx);
                    }
                });
            }
        })
        .on_mouse_up(MouseButton::Left, move |_, _, cx| {
            release_input.update(cx, |input, _| input.dragging = false);
        })
        .when_some(placeholder, |el, placeholder| {
            el.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(10.))
                    .right(px(8.))
                    .flex()
                    .items_center()
                    .text_size(px(13.))
                    .text_color(palette.faint)
                    .child(div().truncate().child(placeholder)),
            )
        })
        .child(text_element)
}

fn is_path_separator(character: char) -> bool {
    matches!(character, '\\' | '/')
}

fn utf16_range(text: &str, range: Range<usize>) -> Range<usize> {
    text[..range.start].encode_utf16().count()..text[..range.end].encode_utf16().count()
}

fn byte_range(text: &str, range: Range<usize>) -> Range<usize> {
    let byte_offset = |offset| {
        let mut units = 0;
        for (index, character) in text.char_indices() {
            units += character.len_utf16();
            if units > offset {
                return index;
            }
        }
        text.len()
    };
    let start = byte_offset(range.start);
    start..byte_offset(range.end).max(start)
}
