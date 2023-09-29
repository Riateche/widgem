use std::{cmp::max, fmt::Display};

use accesskit::{Action, DefaultActionVerb, NodeBuilder, Role};
use cosmic_text::Attrs;
use winit::event::MouseButton;

use crate::{
    callback::Callback,
    draw::DrawEvent,
    event::{AccessibleEvent, FocusReason, MouseInputEvent},
    event::{CursorLeaveEvent, CursorMoveEvent},
    layout::SizeHint,
    style::button::{ButtonState, ComputedVariantStyle},
    system::send_window_request,
    text_editor::TextEditor,
    types::{Point, Rect},
    window::SetFocusRequest,
};

use super::{Widget, WidgetCommon};

pub struct Button {
    editor: TextEditor,
    // TODO: Option inside callback
    on_clicked: Option<Callback<String>>,
    is_pressed: bool,
    common: WidgetCommon,
}

const MIN_PADDING: Point = Point { x: 1, y: 0 };
const PADDING: Point = Point { x: 10, y: 5 };

impl Button {
    pub fn new(text: impl Display) -> Self {
        let mut common = WidgetCommon::new();
        common.is_focusable = true;
        let mut editor = TextEditor::new(&text.to_string());
        editor.set_cursor_hidden(true);
        Self {
            editor,
            on_clicked: None,
            is_pressed: false,
            common,
        }
    }

    pub fn set_text(&mut self, text: impl Display) {
        println!("button set text");
        self.editor.set_text(&text.to_string(), Attrs::new());
        self.common.size_hint_changed();
        self.common.update();
    }

    pub fn on_clicked(&mut self, callback: Callback<String>) {
        self.on_clicked = Some(callback);
    }

    fn click(&mut self) {
        if let Some(on_clicked) = &self.on_clicked {
            on_clicked.invoke(self.editor.text());
        }
    }

    fn current_variant_style(&self) -> &ComputedVariantStyle {
        let state = if self.common.is_enabled() {
            ButtonState::Enabled {
                focused: self.common.is_focused(),
                mouse_over: self.common.is_mouse_entered,
                pressed: self.is_pressed,
            }
        } else {
            ButtonState::Disabled
        };
        self.common.style().button.variants.get(&state)
    }
}

impl Widget for Button {
    fn on_draw(&mut self, event: DrawEvent) {
        let style = self.current_variant_style().clone();

        event.stroke_and_fill_rounded_rect(
            Rect {
                top_left: Point::default(),
                size: event.rect().size,
            },
            style.border.as_ref(),
            style.background.as_ref(),
        );

        self.editor.set_text_color(style.text_color);
        let editor_pixmap = self.editor.pixmap();
        let padding = Point {
            x: max(0, event.rect().size.x - editor_pixmap.width() as i32) / 2,
            y: max(0, event.rect().size.y - editor_pixmap.height() as i32) / 2,
        };
        event.draw_pixmap(padding, editor_pixmap.as_ref());
    }

    fn on_mouse_input(&mut self, event: MouseInputEvent) -> bool {
        // TODO: only on release, check buttons
        if event.button() == MouseButton::Left {
            self.is_pressed = self.common.is_enabled() && event.state().is_pressed();
            if event.state().is_pressed() {
                self.click();
            }
            self.common.update();
        }

        let mount_point = &self
            .common
            .mount_point
            .as_ref()
            .expect("cannot handle event when unmounted");
        send_window_request(
            mount_point.address.window_id,
            SetFocusRequest {
                widget_id: self.common.id,
                reason: FocusReason::Mouse,
            },
        );
        true
    }

    fn on_cursor_move(&mut self, event: CursorMoveEvent) -> bool {
        if event.is_enter() {
            self.common.update();
        }
        true
    }

    fn on_cursor_leave(&mut self, _event: CursorLeaveEvent) {
        self.common.update();
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
    fn on_accessible(&mut self, event: AccessibleEvent) {
        let mount_point = &self
            .common
            .mount_point
            .as_ref()
            .expect("cannot handle event when unmounted");

        match event.action {
            Action::Default => self.click(),
            Action::Focus => {
                send_window_request(
                    mount_point.address.window_id,
                    SetFocusRequest {
                        widget_id: self.common.id,
                        // TODO: separate reason?
                        reason: FocusReason::Mouse,
                    },
                );
            }
            _ => {}
        }
    }

    fn accessible_node(&mut self) -> Option<accesskit::NodeBuilder> {
        let mut node = NodeBuilder::new(Role::Button);
        node.set_name(self.editor.text().as_str());
        node.add_action(Action::Focus);
        //node.add_action(Action::Default);
        node.set_default_action_verb(DefaultActionVerb::Click);
        Some(node)
    }

    fn size_hint_x(&mut self) -> SizeHint {
        println!("test1 {:?}: {:?}", self.common.id, self.editor.size().x);
        SizeHint {
            min: self.editor.size().x + 2 * MIN_PADDING.x,
            preferred: self.editor.size().x + 2 * PADDING.x,
            is_fixed: true,
        }
    }

    fn size_hint_y(&mut self, _size_x: i32) -> SizeHint {
        // TODO: use size_x, handle multiple lines
        SizeHint {
            min: self.editor.size().y + 2 * MIN_PADDING.y,
            preferred: self.editor.size().y + 2 * PADDING.y,
            is_fixed: true,
        }
    }
}
