use std::fmt::Display;

use cosmic_text::{Action, Attrs, Wrap};
use winit::{
    event::{ElementState, Ime, MouseButton},
    keyboard::Key,
};

use crate::{
    draw::DrawEvent,
    event::{
        CursorMovedEvent, FocusOutEvent, FocusReason, GeometryChangedEvent, ImeEvent,
        KeyboardInputEvent, MountEvent, UnmountEvent, WindowFocusChangedEvent,
    },
    system::{send_window_event, with_system},
    text_editor::TextEditor,
    types::{Point, Rect},
    window::{SetFocusRequest, SetImeCursorAreaRequest},
};

use super::{Widget, WidgetCommon};

pub struct TextInput {
    editor: TextEditor,
    common: WidgetCommon,
}

impl TextInput {
    pub fn new(text: impl Display) -> Self {
        let mut common = WidgetCommon::new();
        common.is_focusable = true;
        common.enable_ime = true;
        let mut editor = TextEditor::new(&text.to_string());
        editor.set_wrap(Wrap::None);
        Self { editor, common }
    }

    pub fn set_text(&mut self, text: impl Display) {
        // TODO: replace line breaks to avoid multiple lines in buffer
        self.editor.set_text(&text.to_string(), Attrs::new());
    }
}

impl Widget for TextInput {
    fn on_geometry_changed(&mut self, event: GeometryChangedEvent) {
        if let Some(new_geometry) = event.new_geometry {
            self.editor.set_size(new_geometry.rect_in_window.size);
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

        event.draw_pixmap(Point::default(), self.editor.pixmap().as_ref());
        if self.common.is_focused {
            if let Some(editor_cursor) = self.editor.cursor_position() {
                // TODO: adjust for editor offset
                // We specify an area below the input because on Windows the IME window obscures the specified area.
                let top_left = Point {
                    x: editor_cursor.x,
                    y: editor_cursor.y + self.editor.line_height().ceil() as i32,
                } + geometry.rect_in_window.top_left;
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
                self.editor.on_mouse_input(event.pos, event.button);
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
            self.editor.action(Action::Drag {
                x: event.pos.x,
                y: event.pos.y,
            });
        }
        true
    }

    fn on_keyboard_input(&mut self, event: KeyboardInputEvent) -> bool {
        //println!("on_keyboard_input, {:?}", event);
        if event.event.state == ElementState::Released {
            return true;
        }

        let mount_point = self
            .common
            .mount_point
            .as_ref()
            .expect("cannot handle event when unmounted");
        let modifiers = mount_point.window.0.borrow().modifiers_state;
        let logical_key = event.event.logical_key;
        // TODO: different commands for macOS?
        let action = match logical_key {
            // TODO: scroll lock?
            Key::Escape => {
                // TODO: cosmic-text for some reason suggests to clear selection on Escape?
                None
            }
            Key::Insert => None, //TODO
            Key::Home => Some(Action::Home),
            Key::Delete => Some(Action::Delete),
            Key::End => Some(Action::End),
            Key::PageDown => Some(Action::PageDown),
            Key::PageUp => Some(Action::PageUp),
            Key::ArrowLeft => {
                if modifiers.shift_key() && self.editor.select_opt().is_none() {
                    self.editor.set_select_opt(Some(self.editor.cursor()));
                }
                if !modifiers.shift_key() && self.editor.select_opt().is_some() {
                    self.editor.set_select_opt(None);
                }
                println!("handle left!");
                Some(Action::Left)
            }
            Key::ArrowUp => Some(Action::Up),
            Key::ArrowRight => Some(Action::Right),
            Key::ArrowDown => Some(Action::Down),
            Key::Backspace => Some(Action::Backspace),
            Key::Enter => Some(Action::Enter),
            // Key::Caret => {
            //     // TODO: what's that?
            //     return true;
            // }
            Key::Copy | Key::Cut | Key::Paste => {
                // TODO
                None
            }
            _ => None,
        };
        // println!(
        //     "ok2.2 selection: {:?}, cursor: {:?}",
        //     editor.select_opt(),
        //     editor.cursor()
        // );
        if let Some(action) = action {
            self.editor.action(action);
            return true;
        }
        // println!("###");
        // for line in &editor.buffer().lines {
        //     println!("ok1 {:?}", line.text());
        //     println!("ok2 {:?}", line.text_without_ime());
        // }
        //editor.buffer_mut().set_redraw(true);

        if let Some(text) = event.event.text {
            // TODO: replace line breaks to avoid multiple lines in buffer
            self.editor.insert_string(&text, None);
            return true;
        }
        false
    }

    // fn on_received_character(&mut self, event: ReceivedCharacterEvent) -> bool {
    //     let system = &mut *self
    //         .common
    //         .mount_point
    //         .as_ref()
    //         .expect("cannot handle event when unmounted")
    //         .system
    //         .0
    //         .borrow_mut();

    //     if let Some(editor) = &mut self.editor {
    //         // TODO: replace line breaks to avoid multiple lines in buffer
    //         editor.action(&mut system.font_system, Action::Insert(event.char));
    //         for line in &editor.buffer().lines {
    //             println!("ok3 {:?}", line.text());
    //         }
    //         // println!("###");
    //         // for line in &editor.buffer().lines {
    //         //     println!("ok1 {:?}", line.text());
    //         //     println!("ok2 {:?}", line.text_without_ime());
    //         // }
    //     }
    //     true
    // }

    fn on_ime(&mut self, event: ImeEvent) -> bool {
        match event.0.clone() {
            Ime::Enabled => {}
            Ime::Preedit(pretext, cursor) => {
                // TODO: can pretext have line breaks?
                self.editor
                    .action(Action::ImeSetPretext { pretext, cursor });
            }
            Ime::Commit(string) => {
                self.editor.action(Action::ImeCommit(string));
            }
            Ime::Disabled => {}
        }
        // println!("###");
        // for line in &editor.buffer().lines {
        //     println!("ok1 {:?}", line.text());
        //     println!("ok2 {:?}", line.text_without_ime());
        // }
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
    }
    fn on_unmount(&mut self, _event: UnmountEvent) {
        self.editor.set_window_id(None);
    }
    fn on_focus_out(&mut self, _event: FocusOutEvent) {
        self.editor.on_focus_out();
    }
    fn on_window_focus_changed(&mut self, event: WindowFocusChangedEvent) {
        self.editor.on_window_focus_changed(event.focused);
    }
}
