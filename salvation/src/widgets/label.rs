use std::fmt::Display;

use anyhow::Result;
use cosmic_text::Attrs;

use crate::{
    draw::DrawEvent, impl_widget_common, layout::SizeHintMode, text_editor::TextEditor,
    types::Point,
};

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
    impl_widget_common!();

    fn handle_draw(&mut self, event: DrawEvent) -> Result<()> {
        event.draw_pixmap(
            Point::default(),
            self.editor.pixmap().as_ref(),
            Default::default(),
        );
        Ok(())
    }

    fn recalculate_size_hint_x(&mut self, _mode: SizeHintMode) -> Result<i32> {
        Ok(self.editor.size().x)
    }

    fn recalculate_size_hint_y(&mut self, _size_x: i32, _mode: SizeHintMode) -> Result<i32> {
        // TODO: use size_x, handle multiple lines
        Ok(self.editor.size().y)
    }
}
