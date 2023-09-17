use std::{
    cmp::{max, min},
    fmt::Display,
    time::Duration,
};

use accesskit::{ActionData, DefaultActionVerb, NodeBuilder, NodeId, Role};
use cosmic_text::{Action, Attrs, Wrap};
use log::warn;
use winit::{
    event::{ElementState, Ime, MouseButton},
    keyboard::Key,
    window::CursorIcon,
};

use crate::{
    accessible,
    draw::DrawEvent,
    event::{
        AccessibleEvent, CursorMovedEvent, FocusInEvent, FocusOutEvent, FocusReason,
        GeometryChangedEvent, ImeEvent, KeyboardInputEvent, MountEvent, MouseInputEvent,
        UnmountEvent, WindowFocusChangedEvent,
    },
    layout::SizeHint,
    shortcut::standard_shortcuts,
    system::{add_interval, send_window_request, with_system},
    text_editor::TextEditor,
    timer::TimerId,
    types::{Point, Rect, Size},
    window::{SetFocusRequest, SetImeCursorAreaRequest},
};

use super::{Widget, WidgetCommon, WidgetExt};

const PADDING: i32 = 5;
const MIN_ASPECT_RATIO: i32 = 2;
const PREFERRED_ASPECT_RATIO: i32 = 10;

pub struct TextInput {
    editor: TextEditor,
    editor_viewport_rect: Rect,
    ideal_editor_offset: Point,
    scroll_x: i32,
    common: WidgetCommon,
    blink_timer: Option<TimerId>,
    selected_text: String,
    accessible_line_id: NodeId,
}

// TODO: get system setting
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(500);

fn sanitize(text: &str) -> String {
    text.replace('\n', " ")
}

impl TextInput {
    pub fn new(text: impl Display) -> Self {
        let mut common = WidgetCommon::new();
        common.is_focusable = true;
        common.enable_ime = true;
        common.cursor_icon = CursorIcon::Text;
        let mut editor = TextEditor::new(&sanitize(&text.to_string()));
        editor.set_wrap(Wrap::None);
        Self {
            editor,
            common,
            editor_viewport_rect: Rect::default(),
            // TODO: get from theme
            ideal_editor_offset: Point { x: 5, y: 5 },
            scroll_x: 0,
            blink_timer: None,
            selected_text: String::new(),
            accessible_line_id: accessible::new_id(),
        }
    }

    pub fn set_text(&mut self, text: impl Display) {
        // TODO: replace line breaks to avoid multiple lines in buffer
        self.editor
            .set_text(&sanitize(&text.to_string()), Attrs::new());
        self.after_change();
        self.reset_blink_timer();
    }

    fn after_change(&mut self) {
        self.adjust_scroll();
        let new_selected_text = self.editor.selected_text().unwrap_or_default();
        if new_selected_text != self.selected_text {
            self.selected_text = new_selected_text;
            #[cfg(all(
                unix,
                not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
            ))]
            {
                use arboard::{LinuxClipboardKind, SetExtLinux};

                if !self.selected_text.is_empty() {
                    let r = with_system(|system| {
                        system
                            .clipboard
                            .set()
                            .clipboard(LinuxClipboardKind::Primary)
                            .text(&self.selected_text)
                    });
                    if let Err(err) = r {
                        warn!("clipboard error: {err}");
                    }
                }
            }
        }
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

    fn copy_to_clipboard(&mut self) {
        if let Some(text) = self.editor.selected_text() {
            if let Err(err) = with_system(|system| system.clipboard.set_text(text)) {
                warn!("clipboard error: {err}");
            }
        }
    }

    fn handle_main_click(&mut self, event: MouseInputEvent) {
        let mount_point = &self
            .common
            .mount_point
            .as_ref()
            .expect("cannot handle event when unmounted");

        if !self.common.is_focused {
            send_window_request(
                mount_point.address.window_id,
                SetFocusRequest {
                    widget_id: self.common.id,
                    reason: FocusReason::Mouse,
                },
            );
        }
        self.editor.on_mouse_input(
            event.pos - self.editor_viewport_rect.top_left + Point::new(self.scroll_x, 0),
            event.num_clicks,
            mount_point.window.0.borrow().modifiers_state.shift_key(),
        );
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
            warn!("no geometry in draw event");
            return;
        };

        let mount_point = &self
            .common
            .mount_point
            .as_ref()
            .expect("cannot draw when unmounted");

        with_system(|system| {
            event.stroke_rounded_rect(
                Rect {
                    top_left: Point::default(),
                    size: geometry.rect_in_window.size,
                },
                2.0,
                if self.common.is_focused {
                    system.palette.focused_input_border
                } else {
                    system.palette.unfocused_input_border
                },
                1.0,
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
                send_window_request(
                    mount_point.address.window_id,
                    SetImeCursorAreaRequest(Rect { top_left, size }),
                );
            }
        }
    }

    fn on_mouse_input(&mut self, event: MouseInputEvent) -> bool {
        if event.state == ElementState::Pressed {
            match event.button {
                MouseButton::Left => {
                    self.handle_main_click(event);
                }
                MouseButton::Right => {
                    // let builder = WindowBuilder::new()
                    //     .with_title("test_window")
                    //     .with_position(PhysicalPosition::new(100, 10))
                    //     .with_inner_size(PhysicalSize::new(300, 300))
                    //     .with_decorations(false)
                    //     .with_visible(false);
                    // let window =
                    //     WINDOW_TARGET.with(|window_target| builder.build(window_target).unwrap());
                    // let window = Window::new(window, None);
                    // std::mem::forget(window);
                    // println!("ok");
                }
                MouseButton::Middle => {
                    #[cfg(all(
                        unix,
                        not(any(
                            target_os = "macos",
                            target_os = "android",
                            target_os = "emscripten"
                        ))
                    ))]
                    {
                        use arboard::{GetExtLinux, LinuxClipboardKind};

                        self.handle_main_click(event);
                        if !self.editor.is_mouse_interaction_forbidden() {
                            let r = with_system(|system| {
                                system
                                    .clipboard
                                    .get()
                                    .clipboard(LinuxClipboardKind::Primary)
                                    .text()
                            });
                            match r {
                                Ok(text) => {
                                    self.editor.insert_string(&sanitize(&text), None);
                                }
                                Err(err) => {
                                    warn!("clipboard error: {err}");
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        let is_released = self
            .common
            .mount_point
            .as_ref()
            .map_or(false, |mount_point| {
                mount_point
                    .window
                    .0
                    .borrow()
                    .pressed_mouse_buttons
                    .is_empty()
            });
        if is_released {
            self.editor.mouse_released();
        }
        self.after_change();
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
        self.after_change();
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
            self.copy_to_clipboard();
            self.editor.action(Action::Delete, false);
        } else if shortcuts.copy.matches(&event) {
            self.copy_to_clipboard();
        } else if shortcuts.paste.matches(&event) {
            let r = with_system(|system| system.clipboard.get_text());
            match r {
                Ok(text) => self.editor.insert_string(&sanitize(&text), None),
                Err(err) => warn!("clipboard error: {err}"),
            }
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
        } else if let Some(text) = event.event.text {
            if [Key::Tab, Key::Enter, Key::Escape].contains(&event.event.logical_key) {
                return false;
            }
            self.editor.insert_string(&sanitize(&text), None);
        } else {
            return false;
        }
        self.after_change();
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
                        preedit: sanitize(&preedit),
                        cursor,
                        attrs: None,
                    },
                    false,
                );
            }
            Ime::Commit(string) => {
                self.editor.insert_string(&sanitize(&string), None);
            }
            Ime::Disabled => {}
        }
        // println!("###");
        // for line in &editor.buffer().lines {
        //     println!("ok1 {:?}", line.text());
        //     println!("ok2 {:?}", line.text_without_ime());
        // }
        self.after_change();
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

        event.0.window.0.borrow_mut().accessible_nodes.mount(
            Some(self.common.id.into()),
            self.accessible_line_id,
            0,
        );
    }
    fn on_unmount(&mut self, _event: UnmountEvent) {
        let Some(mount_point) = &self.common.mount_point else {
            warn!("on_unmount: no mount point");
            return;
        };
        mount_point
            .window
            .0
            .borrow_mut()
            .accessible_nodes
            .unmount(Some(self.common.id.into()), self.accessible_line_id);
        mount_point
            .window
            .0
            .borrow_mut()
            .accessible_nodes
            .update(self.accessible_line_id, None);
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
    fn on_accessible(&mut self, event: AccessibleEvent) {
        let mount_point = &self
            .common
            .mount_point
            .as_ref()
            .expect("cannot handle event when unmounted");

        println!("action in text input widget {:?}", event.action);
        match event.action {
            accesskit::Action::Default | accesskit::Action::Focus => {
                send_window_request(
                    mount_point.address.window_id,
                    SetFocusRequest {
                        widget_id: self.common.id,
                        // TODO: separate reason?
                        reason: FocusReason::Mouse,
                    },
                );
            }
            accesskit::Action::SetTextSelection => {
                let Some(ActionData::SetTextSelection(data)) = event.data else {
                    warn!("expected SetTextSelection in data, got {:?}", event.data);
                    return;
                };
                self.editor.set_accessible_selection(data);
                self.after_change();
                self.reset_blink_timer();
            }
            _ => {}
        }
    }
    fn accessible_node(&mut self) -> Option<accesskit::NodeBuilder> {
        let Some(mount_point) = self.common.mount_point.as_ref() else {
            return None;
        };
        let mut line_node = NodeBuilder::new(Role::InlineTextBox);
        let mut line = self.editor.acccessible_line();
        for pos in &mut line.character_positions {
            *pos -= self.scroll_x as f32;
        }
        // let line = AccessibleLine {
        //     text: "abcd🦀".into(),
        //     text_direction: accesskit::TextDirection::LeftToRight,
        //     character_lengths: vec![1, 1, 1, 1, 4],
        //     character_positions: vec![0.0, 20.0, 30.0, 40.0, 50.0],
        //     character_widths: vec![10.0, 10.0, 10.0, 10.0, 10.0],
        //     word_lengths: vec![4],
        //     // line_top: todo!(),
        //     // line_bottom: todo!(),
        // };
        // println!("line data {:?}", line);
        line_node.set_text_direction(line.text_direction);
        line_node.set_value(line.text);
        line_node.set_character_lengths(line.character_lengths);
        line_node.set_character_positions(line.character_positions);
        line_node.set_character_widths(line.character_widths);
        line_node.set_word_lengths(line.word_lengths);

        if let Some(geometry) = self.common.geometry {
            let rect = self
                .editor_viewport_rect
                .translate(geometry.rect_in_window.top_left);
            line_node.set_bounds(accesskit::Rect {
                x0: rect.top_left.x as f64,
                y0: rect.top_left.y as f64,
                x1: rect.bottom_right().x as f64,
                y1: rect.bottom_right().y as f64,
            });
        }

        mount_point
            .window
            .0
            .borrow_mut()
            .accessible_nodes
            .update(self.accessible_line_id, Some(line_node));

        let mut node = NodeBuilder::new(Role::TextInput);
        // TODO: use label
        node.set_name("some input");
        node.add_action(accesskit::Action::Focus);
        node.set_default_action_verb(DefaultActionVerb::Click);
        node.set_text_selection(self.editor.accessible_selection(self.accessible_line_id));
        Some(node)
    }

    fn size_hint_x(&mut self) -> SizeHint {
        let size_hint_y = self.size_hint_y(0);
        SizeHint {
            min: size_hint_y.min * MIN_ASPECT_RATIO,
            preferred: size_hint_y.preferred * PREFERRED_ASPECT_RATIO,
            is_fixed: false,
        }
    }

    fn size_hint_y(&mut self, _size_x: i32) -> SizeHint {
        let size = self.editor.size().y + 2 * PADDING;
        SizeHint {
            min: self.editor.size().y,
            preferred: size,
            is_fixed: true,
        }
    }
}
