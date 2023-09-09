use std::{
    cmp::{max, min},
    fmt::Display,
    time::Duration,
};

use cosmic_text::{Action, Attrs, Wrap};
use winit::{
    event::{ElementState, Ime, MouseButton},
    keyboard::Key,
};

use crate::{
    draw::DrawEvent,
    event::{
        CursorMovedEvent, FocusInEvent, FocusOutEvent, FocusReason, GeometryChangedEvent, ImeEvent,
        KeyboardInputEvent, MountEvent, UnmountEvent, WindowFocusChangedEvent,
    },
    shortcut::standard_shortcuts,
    system::{add_interval, send_window_event, with_system},
    text_editor::TextEditor,
    timer::TimerId,
    types::{Point, Rect, Size},
    window::{SetFocusRequest, SetImeCursorAreaRequest},
};

use super::{Widget, WidgetCommon, WidgetExt};

pub struct TextInput {
    editor: TextEditor,
    editor_viewport_rect: Rect,
    ideal_editor_offset: Point,
    scroll_x: i32,
    common: WidgetCommon,
    blink_timer: Option<TimerId>,
}

// TODO: get system setting
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(500);

impl TextInput {
    pub fn new(text: impl Display) -> Self {
        let mut common = WidgetCommon::new();
        common.is_focusable = true;
        common.enable_ime = true;
        let mut editor = TextEditor::new(&text.to_string());
        editor.set_wrap(Wrap::None);
        Self {
            editor,
            common,
            editor_viewport_rect: Rect::default(),
            // TODO: get from theme
            ideal_editor_offset: Point { x: 5, y: 5 },
            scroll_x: 0,
            blink_timer: None,
        }
    }

    pub fn set_text(&mut self, text: impl Display) {
        // TODO: replace line breaks to avoid multiple lines in buffer
        self.editor.set_text(&text.to_string(), Attrs::new());
        self.reset_blink_timer();
    }

    fn adjust_scroll(&mut self) {
        let max_scroll = max(0, self.editor.size().x - self.editor_viewport_rect.size.x);
        if let Some(cursor_position) = self.editor.cursor_position() {
            let cursor_x_in_viewport = cursor_position.x - self.scroll_x;
            if cursor_x_in_viewport < 0 {
                self.scroll_x -= -cursor_x_in_viewport;
            } else if cursor_x_in_viewport > self.editor_viewport_rect.size.x - 1 {
                self.scroll_x += cursor_x_in_viewport - (self.editor_viewport_rect.size.x - 1);
            }
        }
        self.scroll_x = self.scroll_x.clamp(0, max_scroll);
    }

    fn reset_blink_timer(&mut self) {
        if let Some(id) = self.blink_timer.take() {
            id.cancel();
        }
        let Some(mount_point) = &self.common.mount_point else {
            return;
        };
        let focused = self.common.is_focused && mount_point.window.0.borrow().is_window_focused;
        self.editor.set_cursor_hidden(!focused);
        if focused {
            let id = add_interval(CURSOR_BLINK_INTERVAL, self.id(), |this, _| {
                this.toggle_cursor_hidden();
            });
            self.blink_timer = Some(id);
        }
    }

    fn toggle_cursor_hidden(&mut self) {
        self.editor
            .set_cursor_hidden(!self.editor.is_cursor_hidden());
    }
}

impl Widget for TextInput {
    fn on_geometry_changed(&mut self, event: GeometryChangedEvent) {
        if let Some(new_geometry) = event.new_geometry {
            let offset_y = max(0, new_geometry.rect_in_window.size.y - self.editor.size().y) / 2;
            self.editor_viewport_rect = Rect {
                top_left: Point {
                    x: self.ideal_editor_offset.x,
                    y: offset_y,
                },
                size: Size {
                    x: max(
                        0,
                        new_geometry.rect_in_window.size.x - 2 * self.ideal_editor_offset.x,
                    ),
                    y: min(new_geometry.rect_in_window.size.y, self.editor.size().y),
                },
            };
            self.adjust_scroll();
            self.reset_blink_timer();
            // self.editor.set_size(new_geometry.rect_in_window.size);
        }
    }

    fn on_draw(&mut self, event: DrawEvent) {
        let Some(geometry) = self.common.geometry else {
            println!("warn: no geometry in draw event");
            return;
        };

        let mount_point = &self
            .common
            .mount_point
            .as_ref()
            .expect("cannot draw when unmounted");

        with_system(|system| {
            event.stroke_rect(
                Rect {
                    top_left: Point::default(),
                    size: geometry.rect_in_window.size,
                },
                if self.common.is_focused {
                    system.palette.focused_input_border
                } else {
                    system.palette.unfocused_input_border
                },
            );
        });

        let mut target_rect = self.editor_viewport_rect;
        target_rect.size.x = min(target_rect.size.x, self.editor.size().x);

        let scroll = Point::new(self.scroll_x, 0);
        event.draw_subpixmap(target_rect, self.editor.pixmap().as_ref(), scroll);
        if self.common.is_focused {
            if let Some(editor_cursor) = self.editor.cursor_position() {
                // We specify an area below the input because on Windows
                // the IME window obscures the specified area.
                let top_left =
                    geometry.rect_in_window.top_left + self.editor_viewport_rect.top_left - scroll
                        + editor_cursor
                        + Point {
                            x: 0,
                            y: self.editor.line_height().ceil() as i32,
                        };
                let size = geometry.rect_in_window.size; // TODO: not actually correct
                send_window_event(
                    mount_point.address.window_id,
                    SetImeCursorAreaRequest(Rect { top_left, size }),
                );
            }
        }
    }

    fn on_mouse_input(&mut self, event: crate::event::MouseInputEvent) -> bool {
        let mount_point = self
            .common
            .mount_point
            .as_ref()
            .expect("cannot handle event when unmounted");
        if event.state == ElementState::Pressed {
            if event.button == MouseButton::Left {
                if !self.common.is_focused {
                    send_window_event(
                        mount_point.address.window_id,
                        SetFocusRequest {
                            widget_id: self.common.id,
                            reason: FocusReason::Mouse,
                        },
                    );
                }
                self.editor.on_mouse_input(
                    event.pos - self.editor_viewport_rect.top_left + Point::new(self.scroll_x, 0),
                    event.button,
                    event.num_clicks,
                    mount_point.window.0.borrow().modifiers_state.shift_key(),
                );

                // add_timer(Duration::from_secs(1), self.id(), |this, _| {
                //     this.update_blink();
                // });
            }
            if event.button == MouseButton::Right {
                // let builder = WindowBuilder::new()
                //     .with_title("test_window")
                //     .with_position(PhysicalPosition::new(100, 10))
                //     .with_inner_size(PhysicalSize::new(300, 300))
                //     .with_decorations(false)
                //     .with_visible(true);
                // let window =
                //     WINDOW_TARGET.with(|window_target| builder.build(window_target).unwrap());
                // let window = Window::new(window, None);
                // std::mem::forget(window);
                // println!("ok");
            }
        }
        self.adjust_scroll();
        self.reset_blink_timer();
        true
    }

    fn on_cursor_moved(&mut self, event: CursorMovedEvent) -> bool {
        let mount_point = self
            .common
            .mount_point
            .as_ref()
            .expect("cannot handle event when unmounted");
        if mount_point
            .window
            .0
            .borrow()
            .pressed_mouse_buttons
            .contains(&MouseButton::Left)
        {
            let pos = event.pos - self.editor_viewport_rect.top_left + Point::new(self.scroll_x, 0);
            self.editor
                .action(Action::Drag { x: pos.x, y: pos.y }, true);
        }
        self.adjust_scroll();
        true
    }

    #[allow(clippy::if_same_then_else)]
    fn on_keyboard_input(&mut self, event: KeyboardInputEvent) -> bool {
        // println!("text input on_keyboard_input, {:?}", event);
        if event.event.state == ElementState::Released {
            return true;
        }

        let shortcuts = standard_shortcuts();
        if shortcuts.move_to_next_char.matches(&event) {
            self.editor.action(Action::Next, false);
        } else if shortcuts.move_to_previous_char.matches(&event) {
            self.editor.action(Action::Previous, false);
        } else if shortcuts.delete.matches(&event) {
            self.editor.action(Action::Delete, false);
        } else if shortcuts.backspace.matches(&event) {
            self.editor.action(Action::Backspace, false);
        } else if shortcuts.cut.matches(&event) {
            // TODO
        } else if shortcuts.copy.matches(&event) {
            // TODO
        } else if shortcuts.paste.matches(&event) {
            // TODO
        } else if shortcuts.undo.matches(&event) {
            // TODO
        } else if shortcuts.redo.matches(&event) {
            // TODO
        } else if shortcuts.select_all.matches(&event) {
            self.editor.action(Action::SelectAll, false);
        } else if shortcuts.deselect.matches(&event) {
            // TODO: why Escape?
            self.editor.action(Action::Escape, false);
        } else if shortcuts.move_to_next_word.matches(&event) {
            self.editor.action(Action::NextWord, false);
        } else if shortcuts.move_to_previous_word.matches(&event) {
            self.editor.action(Action::PreviousWord, false);
        } else if shortcuts.move_to_start_of_line.matches(&event) {
            self.editor.action(Action::Home, false);
        } else if shortcuts.move_to_end_of_line.matches(&event) {
            self.editor.action(Action::End, false);
        } else if shortcuts.select_next_char.matches(&event) {
            self.editor.action(Action::Next, true);
        } else if shortcuts.select_previous_char.matches(&event) {
            self.editor.action(Action::Previous, true);
        } else if shortcuts.select_next_word.matches(&event) {
            self.editor.action(Action::NextWord, true);
        } else if shortcuts.select_previous_word.matches(&event) {
            self.editor.action(Action::PreviousWord, true);
        } else if shortcuts.select_start_of_line.matches(&event) {
            self.editor.action(Action::Home, true);
        } else if shortcuts.select_end_of_line.matches(&event) {
            self.editor.action(Action::End, true);
        } else if shortcuts.delete_start_of_word.matches(&event) {
            self.editor.action(Action::DeleteStartOfWord, false);
        } else if shortcuts.delete_end_of_word.matches(&event) {
            self.editor.action(Action::DeleteEndOfWord, false);
        } else if shortcuts.insert_paragraph_separator.matches(&event) {
            self.editor.action(Action::Enter, false);
        } else if let Some(text) = event.event.text {
            if event.event.logical_key == Key::Tab {
                return false;
            }
            self.editor.insert_string(&text, None);
        } else {
            return false;
        }
        self.adjust_scroll();
        self.reset_blink_timer();
        true
    }

    fn on_ime(&mut self, event: ImeEvent) -> bool {
        match event.0.clone() {
            Ime::Enabled => {}
            Ime::Preedit(preedit, cursor) => {
                // TODO: can pretext have line breaks?
                self.editor.action(
                    Action::SetPreedit {
                        preedit,
                        cursor,
                        attrs: None,
                    },
                    false,
                );
            }
            Ime::Commit(string) => {
                self.editor.insert_string(&string, None);
            }
            Ime::Disabled => {}
        }
        // println!("###");
        // for line in &editor.buffer().lines {
        //     println!("ok1 {:?}", line.text());
        //     println!("ok2 {:?}", line.text_without_ime());
        // }
        self.adjust_scroll();
        self.reset_blink_timer();
        true
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn on_mount(&mut self, event: MountEvent) {
        self.editor.set_window_id(Some(event.0.address.window_id));
        self.reset_blink_timer();
    }
    fn on_unmount(&mut self, _event: UnmountEvent) {
        self.editor.set_window_id(None);
        self.reset_blink_timer();
    }
    fn on_focus_in(&mut self, event: FocusInEvent) {
        self.editor.on_focus_in(event.reason);
        self.reset_blink_timer();
    }
    fn on_focus_out(&mut self, _event: FocusOutEvent) {
        self.editor.on_focus_out();
        self.reset_blink_timer();
    }
    fn on_window_focus_changed(&mut self, event: WindowFocusChangedEvent) {
        self.editor.on_window_focus_changed(event.focused);
        self.reset_blink_timer();
    }
}
