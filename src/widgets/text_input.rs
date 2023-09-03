use std::fmt::Display;

use cosmic_text::{Action, Attrs, Buffer, Edit, Editor, Shaping, Wrap};
use tiny_skia::Pixmap;
use winit::{event::{ElementState, Ime, MouseButton}, keyboard::Key};

use crate::{
    draw::{draw_text, DrawEvent},
    event::{CursorMovedEvent, FocusReason, ImeEvent, KeyboardInputEvent},
    event_loop::UserEvent,
    types::{Point, Rect, Size},
    window::{SetFocusRequest, SetImePositionRequest, WindowRequest},
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
        let mut common = WidgetCommon::new();
        common.is_focusable = true;
        common.enable_ime = true;
        Self {
            text: text.to_string(),
            editor: None,
            pixmap: None,
            redraw_text: true,
            common,
        }
    }

    pub fn set_text(&mut self, text: impl Display) {
        self.text = text.to_string();
        // TODO: update buffer
        // TODO: replace line breaks to avoid multiple lines in buffer
        self.redraw_text = true;
    }
}

impl Widget for TextInput {
    fn on_draw(&mut self, event: DrawEvent) {
        // let mut pb = PathBuilder::new();
        // pb.move_to(20.5, 20.5);
        // pb.line_to(220.5, 20.5);
        // for i in 1..=8 {
        //     let angle2 = PI / 2.0 / 8.0 * (i as f32);
        //     let angle1 = angle2 - PI / 16.0;
        //     pb.quad_to(
        //         220.5 + angle1.sin() * 50.0,
        //         70.5 - angle1.cos() * 50.0,
        //         220.5 + angle2.sin() * 50.0,
        //         70.5 - angle2.cos() * 50.0,
        //     );
        //     // pb.line_to(220.5 + angle1.sin() * 50.0, 70.5 - angle1.cos() * 50.0);
        //     // pb.line_to(220.5 + angle2.sin() * 50.0, 70.5 - angle2.cos() * 50.0);
        // }
        //pb.push_circle(x, y, r)

        // let path =
        //     PathBuilder::from_rect(tiny_skia::Rect::from_xywh(20.5, 20.5, 300.0, 40.0).unwrap());
        // ctx.pixmap.stroke_path(
        //     &pb.finish().unwrap(),
        //     &Paint {
        //         shader: tiny_skia::Shader::SolidColor(Color::from_rgba8(0, 0, 0, 255)),
        //         ..Paint::default()
        //     },
        //     &Stroke {
        //         width: 1.0,
        //         line_join: tiny_skia::LineJoin::Round,
        //         ..Stroke::default()
        //     },
        //     Transform::default(),
        //     None,
        // );
        let Some(geometry) = self.common.geometry else {
            println!("warn: no geometry in draw event");
            return;
        };

        let mount_point = &self
            .common
            .mount_point
            .as_ref()
            .expect("cannot draw when unmounted");
        let system = &mut *mount_point.system.0.borrow_mut();

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

        // let editor_top_left = Point {
        //     x: 10, y: 10,
        // };

        let mut editor = self
            .editor
            .get_or_insert_with(|| {
                let mut editor =
                    Editor::new(Buffer::new(&mut system.font_system, system.font_metrics));
                editor
                    .buffer_mut()
                    .set_wrap(&mut system.font_system, Wrap::None);
                editor.buffer_mut().set_text(
                    &mut system.font_system,
                    &self.text,
                    Attrs::new(),
                    Shaping::Advanced,
                );
                editor
                    .buffer_mut()
                    .set_size(&mut system.font_system, 300.0, 30.0);
                editor
            })
            .borrow_with(&mut system.font_system);

        editor.shape_as_needed();
        if editor.buffer().redraw() {
            let size = Size {
                x: editor.buffer().size().0.ceil() as i32,
                y: editor.buffer().size().1.ceil() as i32,
            };
            //println!("redraw ok2 {:?}, {}", size, editor.buffer().scroll());
            // let pixmap = draw_text(&mut editor, size, ctx.palette.foreground, ctx.swash_cache);
            let pixmap = draw_text(
                &mut editor,
                size,
                system.palette.foreground,
                &mut system.swash_cache,
            );
            self.pixmap = Some(pixmap);
            self.redraw_text = false;
            editor.buffer_mut().set_redraw(false);
            if self.common.is_focused {
                if let Some((editor_cursor_x, editor_cursor_y)) = editor.cursor_position() {
                    // TODO: adjust for editor offset
                    let pos = Point {
                        x: editor_cursor_x,
                        y: editor_cursor_y + editor.buffer().metrics().line_height.ceil() as i32,
                    } + geometry.rect_in_window.top_left;
                    let _ = system.event_loop_proxy.send_event(UserEvent::WindowRequest(
                        mount_point.address.window_id,
                        WindowRequest::SetImePosition(SetImePositionRequest(pos)),
                    ));
                }
            }
        }

        if let Some(pixmap) = &self.pixmap {
            event.draw_pixmap(Point::default(), pixmap.as_ref());
        }
    }

    fn on_mouse_input(&mut self, event: crate::event::MouseInputEvent) -> bool {
        let mount_point = self
            .common
            .mount_point
            .as_ref()
            .expect("cannot handle event when unmounted");
        let system = &mut *mount_point.system.0.borrow_mut();

        if event.state == ElementState::Pressed {
            if event.button == MouseButton::Left {
                if !self.common.is_focused {
                    let _ = system.event_loop_proxy.send_event(UserEvent::WindowRequest(
                        mount_point.address.window_id,
                        WindowRequest::SetFocus(SetFocusRequest {
                            widget_id: self.common.id,
                            reason: FocusReason::Mouse,
                        }),
                    ));
                }
                if let Some(editor) = &mut self.editor {
                    editor.action(
                        &mut system.font_system,
                        Action::Click {
                            x: event.pos.x,
                            y: event.pos.y,
                        },
                    );
                }
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
        let system = &mut *mount_point.system.0.borrow_mut();

        if mount_point
            .window
            .0
            .borrow()
            .pressed_mouse_buttons
            .contains(&MouseButton::Left)
        {
            if let Some(editor) = &mut self.editor {
                editor.action(
                    &mut system.font_system,
                    Action::Drag {
                        x: event.pos.x,
                        y: event.pos.y,
                    },
                );
                println!(
                    "ok after drag selection: {:?}, cursor: {:?}",
                    editor.select_opt(),
                    editor.cursor()
                );
            }
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
        let system = &mut *mount_point.system.0.borrow_mut();
        let modifiers = mount_point.window.0.borrow().modifiers_state;
        if let Some(editor) = &mut self.editor {
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
                    if modifiers.shift_key() && editor.select_opt().is_none() {
                        editor.set_select_opt(Some(editor.cursor()));
                    }
                    if !modifiers.shift_key() && editor.select_opt().is_some() {
                        editor.set_select_opt(None);
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
                _ => {
                    None
                }
            };
            // println!(
            //     "ok2.2 selection: {:?}, cursor: {:?}",
            //     editor.select_opt(),
            //     editor.cursor()
            // );
            println!("before {:?}", editor.cursor());
            if let Some(action) = action {
                editor.action(&mut system.font_system, action);
                return true;
            }
            println!("after {:?}", editor.cursor());
            // println!("###");
            // for line in &editor.buffer().lines {
            //     println!("ok1 {:?}", line.text());
            //     println!("ok2 {:?}", line.text_without_ime());
            // }
            //editor.buffer_mut().set_redraw(true);

            if let Some(text) = event.event.text {
                // TODO: replace line breaks to avoid multiple lines in buffer
                editor.insert_string(&text, None);
                for line in &editor.buffer().lines {
                    println!("ok3 {:?}", line.text());
                }
                return true;
            }
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
        let system = &mut *self
            .common
            .mount_point
            .as_ref()
            .expect("cannot handle event when unmounted")
            .system
            .0
            .borrow_mut();

        if let Some(editor) = &mut self.editor {
            match event.0.clone() {
                Ime::Enabled => {}
                Ime::Preedit(pretext, cursor) => {
                    // TODO: can pretext have line breaks?
                    println!("handle ime!");
                    println!("before {:?}", editor.cursor());
                    editor.action(
                        &mut system.font_system,
                        Action::ImeSetPretext { pretext, cursor },
                    );
                    println!("after {:?}", editor.cursor());
                }
                Ime::Commit(string) => {
                    editor.action(&mut system.font_system, Action::ImeCommit(string));
                }
                Ime::Disabled => {}
            }
            // println!("###");
            // for line in &editor.buffer().lines {
            //     println!("ok1 {:?}", line.text());
            //     println!("ok2 {:?}", line.text_without_ime());
            // }
        }
        true
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
}
