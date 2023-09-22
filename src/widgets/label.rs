use std::fmt::Display;

use cosmic_text::Attrs;

use crate::{draw::DrawEvent, layout::SizeHint, text_editor::TextEditor, types::Point};

use super::{Widget, WidgetCommon};

pub struct Label {
    editor: TextEditor,
    common: WidgetCommon,
}

impl Label {
    pub fn new(text: impl Display) -> Self {
        let mut editor = TextEditor::new(&text.to_string());
        editor.set_cursor_hidden(true);
        Self {
            editor,
            common: WidgetCommon::new(),
        }
    }

    pub fn set_text(&mut self, text: impl Display) {
        self.editor.set_text(&text.to_string(), Attrs::new());
        self.common.size_hint_changed();
        self.common.update();
    }
}

impl Widget for Label {
    fn on_draw(&mut self, event: DrawEvent) {
        event.draw_pixmap(Point::default(), self.editor.pixmap().as_ref());
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn size_hint_x(&mut self) -> SizeHint {
        SizeHint {
            min: self.editor.size().x,
            preferred: self.editor.size().x,
            is_fixed: true,
        }
    }

    fn size_hint_y(&mut self, _size_x: i32) -> SizeHint {
        // TODO: use size_x, handle multiple lines
        SizeHint {
            min: self.editor.size().y,
            preferred: self.editor.size().y,
            is_fixed: true,
        }
    }
}
