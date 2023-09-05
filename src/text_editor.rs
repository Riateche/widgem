use std::ops::Range;

use cosmic_text::{
    Action, Attrs, AttrsList, AttrsOwned, Buffer, Cursor, Edit, Editor, Shaping, Wrap,
};
use tiny_skia::{Color, Paint, Pixmap, Transform};
use winit::{event::MouseButton, window::WindowId};

use crate::{
    draw::convert_color,
    system::{send_window_event, with_system},
    types::{Point, Size},
    window::CancelImePreedit,
};

pub struct TextEditor {
    editor: Editor,
    pixmap: Option<Pixmap>,
    text_color: Color,
    size: Size,
    window_id: Option<WindowId>,
}

impl TextEditor {
    pub fn new(text: &str) -> Self {
        let mut e = with_system(|system| Self {
            editor: Editor::new(Buffer::new(&mut system.font_system, system.font_metrics)),
            pixmap: None,
            text_color: system.palette.foreground,
            size: Size::default(),
            window_id: None,
        });
        e.set_text(text, Attrs::new());
        e
    }

    pub fn set_window_id(&mut self, window_id: Option<WindowId>) {
        self.window_id = window_id;
    }

    pub fn set_wrap(&mut self, wrap: Wrap) {
        with_system(|system| {
            self.editor
                .buffer_mut()
                .set_wrap(&mut system.font_system, wrap);
        });
    }

    pub fn set_text(&mut self, text: &str, attrs: Attrs) {
        with_system(|system| {
            self.editor.buffer_mut().set_text(
                &mut system.font_system,
                text,
                attrs,
                Shaping::Advanced,
            );
        });
    }
    pub fn insert_string(&mut self, text: &str, attrs_list: Option<AttrsList>) {
        self.editor.insert_string(text, attrs_list)
    }

    pub fn set_size(&mut self, size: Size) {
        with_system(|system| {
            self.editor.buffer_mut().set_size(
                &mut system.font_system,
                size.x as f32,
                size.y as f32,
            );
        });
        self.size = size;
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn set_text_color(&mut self, color: Color) {
        self.text_color = color;
        self.editor.buffer_mut().set_redraw(true);
    }

    pub fn shape_as_needed(&mut self) {
        with_system(|system| self.editor.shape_as_needed(&mut system.font_system));
    }

    pub fn redraw(&mut self) -> bool {
        self.shape_as_needed();
        self.editor.buffer().redraw()
    }

    pub fn pixmap(&mut self) -> &Pixmap {
        if self.pixmap.is_none() || self.redraw() {
            let size = Size {
                x: self.editor.buffer().size().0.ceil() as i32,
                y: self.editor.buffer().size().1.ceil() as i32,
            };
            // TODO: empty size check
            // TODO: error propagation?
            let mut pixmap =
                Pixmap::new(size.x as u32, size.y as u32).expect("failed to create pixmap");
            with_system(|system| {
                self.editor.draw(
                    &mut system.font_system,
                    &mut system.swash_cache,
                    convert_color(self.text_color),
                    |x, y, w, h, c| {
                        let color = Color::from_rgba8(c.r(), c.g(), c.b(), c.a());
                        let paint = Paint {
                            shader: tiny_skia::Shader::SolidColor(color),
                            ..Paint::default()
                        };
                        pixmap.fill_rect(
                            tiny_skia::Rect::from_xywh(x as f32, y as f32, w as f32, h as f32)
                                .unwrap(),
                            &paint,
                            Transform::default(),
                            None,
                        );
                    },
                );
            });
            self.pixmap = Some(pixmap);
            self.editor.buffer_mut().set_redraw(false);
        }
        self.pixmap.as_ref().expect("created above")
    }

    pub fn cursor_position(&mut self) -> Option<Point> {
        self.editor.cursor_position().map(|(x, y)| Point { x, y })
    }

    pub fn line_height(&self) -> f32 {
        self.editor.buffer().metrics().line_height
    }

    // TODO: remove
    pub fn action(&mut self, action: Action) {
        with_system(|system| self.editor.action(&mut system.font_system, action));
    }

    pub fn cursor(&self) -> Cursor {
        self.editor.cursor()
    }
    pub fn select_opt(&self) -> Option<Cursor> {
        self.editor.select_opt()
    }
    pub fn set_select_opt(&mut self, select_opt: Option<Cursor>) {
        self.editor.set_select_opt(select_opt);
    }

    fn ime_range(&self) -> Option<Range<usize>> {
        let cursor = self.editor.cursor();
        let line = &self.editor.buffer().lines[cursor.line];
        line.attrs_list()
            .spans()
            .iter()
            .find(|(_, attrs)| attrs.is_ime_pretext)
            .map(|(range, _)| (*range).clone())
    }

    fn interrupt_ime(&mut self) {
        let cursor = self.editor.cursor();
        let ime_range = self.ime_range();
        let line = &mut self.editor.buffer_mut().lines[cursor.line];
        if let Some(ime_range) = ime_range {
            let mut attrs_list = line.attrs_list().clone();
            let defaults = AttrsOwned::new(attrs_list.defaults());
            attrs_list.add_span(ime_range, defaults.as_attrs());
            line.set_attrs_list(attrs_list);
            self.shape_as_needed();
            if let Some(window_id) = self.window_id {
                send_window_event(window_id, CancelImePreedit);
            } else {
                println!("warn: no window id in text editor event handler");
            }
        }
    }

    pub fn on_focus_out(&mut self) {
        self.interrupt_ime();
        // TODO: hide cursor
    }

    pub fn on_window_focus_changed(&mut self, focused: bool) {
        if !focused {
            self.interrupt_ime();
        }
    }

    pub fn on_mouse_input(&mut self, pos: Point, button: MouseButton) {
        match button {
            MouseButton::Left => {
                let ime_range = self.ime_range();
                let old_cursor = self.editor.cursor();
                let click_cursor = self.editor.buffer().hit(pos.x as f32, pos.y as f32);
                if let Some(click_cursor) = click_cursor {
                    if click_cursor.line == old_cursor.line
                        && ime_range
                            .as_ref()
                            .map_or(false, |ime_range| ime_range.contains(&click_cursor.index))
                    {
                        // Click is inside IME preedit, so we ignore it.
                        println!("click inside ime");
                    } else {
                        // Click is outside IME preedit, so we insert the preedit text
                        // as real text and cancel IME preedit.
                        self.interrupt_ime();
                        println!("action click");
                        self.action(Action::Click { x: pos.x, y: pos.y });
                    }
                }
            }
            MouseButton::Right => {
                // TODO: context menu
            }
            MouseButton::Middle => {
                // TODO: paste selection
            }
            _ => {}
        }
    }
}

impl Default for TextEditor {
    fn default() -> Self {
        Self::new("")
    }
}
