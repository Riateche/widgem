use std::fmt::Display;

use cosmic_text::{Attrs, Buffer, Edit, Editor, Shaping};
use tiny_skia::{Color, Pixmap};
use winit::event::MouseButton;

use crate::{
    draw::{draw_text, DrawContext},
    event::CursorMovedEvent,
    types::{Point, Size},
};

use super::{Widget, WidgetCommon};

pub struct TextInput {
    text: String,
    editor: Option<Editor>,
    pixmap: Option<Pixmap>,
    redraw_text: bool,
    common: WidgetCommon,
}

impl TextInput {
    pub fn new(text: impl Display) -> Self {
        Self {
            text: text.to_string(),
            editor: None,
            pixmap: None,
            redraw_text: true,
            common: WidgetCommon::new(),
        }
    }

    pub fn set_text(&mut self, text: impl Display) {
        self.text = text.to_string();
        self.redraw_text = true;
    }
}

impl Widget for TextInput {
    fn draw(&mut self, ctx: &mut DrawContext<'_>) {
        let mut editor = self
            .editor
            .get_or_insert_with(|| {
                let mut editor = Editor::new(Buffer::new(ctx.font_system, ctx.font_metrics));
                editor.buffer_mut().set_text(
                    ctx.font_system,
                    &self.text,
                    Attrs::new(),
                    Shaping::Advanced,
                );
                editor.buffer_mut().set_size(ctx.font_system, 300.0, 30.0);
                editor
            })
            .borrow_with(ctx.font_system);

        if editor.buffer().redraw() {
            let size = Size {
                x: editor.buffer().size().0.ceil() as i32,
                y: editor.buffer().size().1.ceil() as i32,
            };
            // let pixmap = draw_text(&mut editor, size, ctx.palette.foreground, ctx.swash_cache);
            let pixmap = draw_text(
                &mut editor,
                size,
                Color::from_rgba8(255, 0, 0, 100),
                ctx.swash_cache,
            );
            self.pixmap = Some(pixmap);
            self.redraw_text = false;
            editor.buffer_mut().set_redraw(false);
        }

        if let Some(pixmap) = &self.pixmap {
            ctx.draw_pixmap(Point::default(), pixmap.as_ref());
        }
    }

    fn mouse_input(&mut self, event: &mut crate::event::MouseInputEvent<'_>) {
        if let Some(editor) = &mut self.editor {
            editor.action(
                event.font_system,
                cosmic_text::Action::Click {
                    x: event.pos.x,
                    y: event.pos.y,
                },
            );
        }
    }

    fn cursor_moved(&mut self, event: &mut CursorMovedEvent<'_>) {
        if event.pressed_mouse_buttons.contains(&MouseButton::Left) {
            if let Some(editor) = &mut self.editor {
                editor.action(
                    event.font_system,
                    cosmic_text::Action::Drag {
                        x: event.pos.x,
                        y: event.pos.y,
                    },
                );
            }
        }
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
}
