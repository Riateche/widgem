use std::cmp::max;

use cosmic_text::{
    Action, Attrs, AttrsList, AttrsOwned, Buffer, Cursor, Edit, Editor, Shaping, Wrap,
};
use line_straddler::{GlyphStyle, LineGenerator, LineType};
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Shader, Stroke, Transform};
use winit::window::WindowId;

use crate::{
    draw::{convert_color, unrestricted_text_size},
    event::FocusReason,
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
    is_cursor_hidden: bool,
    forbid_mouse_interaction: bool,
}

impl TextEditor {
    pub fn new(text: &str) -> Self {
        let mut e = with_system(|system| Self {
            editor: Editor::new(Buffer::new(&mut system.font_system, system.font_metrics)),
            pixmap: None,
            text_color: system.palette.foreground,
            size: Size::default(),
            window_id: None,
            is_cursor_hidden: false,
            forbid_mouse_interaction: false,
        });
        e.set_text(text, Attrs::new());
        // TODO: get from theme
        e.editor
            .set_selection_color(Some(cosmic_text::Color::rgb(61, 174, 233)));
        e.editor
            .set_selected_text_color(Some(cosmic_text::Color::rgb(255, 255, 255)));
        e.adjust_size();
        // let mut c = e.cursor();
        // c.color = Some(cosmic_text::Color::rgb(0, 255, 0));
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
        self.adjust_size();
    }

    pub fn text(&self) -> String {
        self.editor.buffer().text_without_preedit()
    }

    pub fn insert_string(&mut self, text: &str, attrs_list: Option<AttrsList>) {
        self.editor.insert_string(text, attrs_list);
        self.adjust_size();
    }

    fn set_size(&mut self, size: Size) {
        if size == self.size {
            return;
        }
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

    pub fn needs_redraw(&mut self) -> bool {
        self.shape_as_needed();
        self.editor.buffer().redraw()
    }

    pub fn is_mouse_interaction_forbidden(&self) -> bool {
        self.forbid_mouse_interaction
    }

    pub fn pixmap(&mut self) -> &Pixmap {
        if self.pixmap.is_none() || self.needs_redraw() {
            let size = Size {
                x: max(1, self.editor.buffer().size().0.ceil() as i32),
                y: max(1, self.editor.buffer().size().1.ceil() as i32),
            };
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
                            shader: Shader::SolidColor(color),
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
            let mut alg = LineGenerator::new(LineType::Underline);
            let mut lines = Vec::new();
            let line_height = self.editor.buffer().metrics().line_height;
            // TODO: determine from glyph width?
            let stroke_width = 1.0;
            for run in self.editor.buffer().layout_runs() {
                let underline_space = line_height - run.line_y;
                let line_y = run.line_top + underline_space / 2.0;
                let line_y = (line_y + stroke_width / 2.0).round() - stroke_width / 2.0;
                for glyph in run.glyphs {
                    if glyph.metadata & 0x1 != 0 {
                        //println!("glyph ok");
                        let color = glyph.color_opt.unwrap_or(convert_color(self.text_color));
                        let glyph = line_straddler::Glyph {
                            line_y,
                            font_size: glyph.font_size,
                            width: glyph.w,
                            x: glyph.x,
                            style: GlyphStyle {
                                boldness: 1,
                                color: line_straddler::Color::rgba(
                                    color.r(),
                                    color.g(),
                                    color.b(),
                                    color.a(),
                                ),
                            },
                        };
                        lines.extend(alg.add_glyph(glyph));
                    }
                }
            }
            lines.extend(alg.pop_line());
            for line in lines {
                //println!("line ok {line:?}");
                let mut path = PathBuilder::new();
                path.move_to(line.start_x, line.y);
                path.line_to(line.end_x, line.y);
                pixmap.stroke_path(
                    &path.finish().unwrap(),
                    &Paint {
                        shader: Shader::SolidColor(tiny_skia::Color::from_rgba8(
                            line.style.color.red(),
                            line.style.color.green(),
                            line.style.color.blue(),
                            line.style.color.alpha(),
                        )),
                        ..Paint::default()
                    },
                    &Stroke {
                        width: stroke_width,
                        ..Stroke::default()
                    },
                    Transform::default(),
                    None,
                );
            }
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
    pub fn action(&mut self, mut action: Action, select: bool) {
        // println!("action {:?}", action);
        match &mut action {
            Action::SetPreedit { attrs, .. } => {
                if attrs.is_none() {
                    let new_attrs = self.attrs_at_cursor().metadata(1);
                    *attrs = Some(AttrsOwned::new(new_attrs));
                }
            }
            Action::Drag { .. } => {
                if self.forbid_mouse_interaction {
                    return;
                }
            }
            _ => (),
        }
        with_system(|system| self.editor.action(&mut system.font_system, action, select));
        self.adjust_size();
    }

    pub fn cursor(&self) -> Cursor {
        self.editor.cursor()
    }
    pub fn set_cursor(&mut self, cursor: Cursor) {
        self.editor.set_cursor(cursor);
    }
    pub fn has_selection(&self) -> bool {
        self.editor.has_selection()
    }
    pub fn select_opt(&self) -> Option<Cursor> {
        self.editor.select_opt()
    }
    pub fn set_select_opt(&mut self, select_opt: Option<Cursor>) {
        self.editor.set_select_opt(select_opt);
    }

    fn interrupt_preedit(&mut self) {
        if let Some(text) = self.editor.preedit_text() {
            let text = text.to_owned();
            self.action(
                Action::SetPreedit {
                    preedit: String::new(),
                    cursor: None,
                    attrs: None,
                },
                false,
            );
            self.insert_string(&text, None);
            if let Some(window_id) = self.window_id {
                send_window_event(window_id, CancelImePreedit);
            } else {
                println!("warn: no window id in text editor event handler");
            }
        }
    }

    pub fn on_focus_in(&mut self, reason: FocusReason) {
        if reason == FocusReason::Tab {
            self.action(Action::SelectAll, false);
        }
    }

    pub fn on_focus_out(&mut self) {
        self.interrupt_preedit();
        self.action(Action::Escape, false);
    }

    pub fn on_window_focus_changed(&mut self, focused: bool) {
        if !focused {
            self.interrupt_preedit();
        }
    }

    pub fn on_mouse_input(&mut self, pos: Point, num_clicks: u32, select: bool) {
        let old_cursor = self.editor.cursor();
        let preedit_range = self.editor.preedit_range();
        let click_cursor = self.editor.buffer().hit(pos.x as f32, pos.y as f32);
        if let Some(click_cursor) = click_cursor {
            if click_cursor.line == old_cursor.line
                && preedit_range
                    .as_ref()
                    .map_or(false, |ime_range| ime_range.contains(&click_cursor.index))
            {
                // Click is inside IME preedit, so we ignore it.
                //println!("click inside ime");
                self.forbid_mouse_interaction = true;
            } else {
                // Click is outside IME preedit, so we insert the preedit text
                // as real text and cancel IME preedit.
                self.interrupt_preedit();
                self.shape_as_needed();
                // println!("action click");
                let x = pos.x;
                let y = pos.y;
                match ((num_clicks - 1) % 3) + 1 {
                    1 => self.action(Action::Click { x, y }, select),
                    2 => self.action(Action::SelectWord { x, y }, false),
                    3 => self.action(Action::SelectParagraph { x, y }, false),
                    _ => {}
                }
            }
        }
    }

    pub fn mouse_released(&mut self) {
        self.forbid_mouse_interaction = false;
    }

    fn attrs_at_cursor(&self) -> Attrs {
        // TODO: use lines.get() everywhere to be safe
        let line = &self.editor.buffer().lines[self.editor.cursor().line];
        line.attrs_list().get_span(self.editor.cursor().index)
    }

    pub fn unrestricted_text_size(&mut self) -> Size {
        with_system(|system| {
            unrestricted_text_size(
                &mut self
                    .editor
                    .buffer_mut()
                    .borrow_with(&mut system.font_system),
            )
        })
    }

    // TODO: adapt for multiline text
    fn adjust_size(&mut self) {
        let unrestricted_size = self.unrestricted_text_size();
        self.set_size(unrestricted_size);
    }

    pub fn set_cursor_hidden(&mut self, hidden: bool) {
        self.editor.set_cursor_hidden(hidden);
        self.is_cursor_hidden = hidden;
    }

    pub fn is_cursor_hidden(&self) -> bool {
        self.is_cursor_hidden
    }

    pub fn selected_text(&mut self) -> Option<String> {
        // TODO: patch cosmic-text to remove mut and don't return empty selection
        self.editor.copy_selection().filter(|s| !s.is_empty())
    }
}

impl Default for TextEditor {
    fn default() -> Self {
        Self::new("")
    }
}
