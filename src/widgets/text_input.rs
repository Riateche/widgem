use std::fmt::Display;

use cosmic_text::{Action, Attrs, Buffer, Edit, Editor, Shaping, Wrap};
use tiny_skia::Pixmap;
use winit::event::{ElementState, Ime, MouseButton, VirtualKeyCode};

use crate::{
    draw::{draw_text, DrawContext},
    event::{CursorMovedEvent, ImeEvent, KeyboardInputEvent, ReceivedCharacterEvent},
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
        let mut common = WidgetCommon::new();
        common.is_focusable = true;
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
    fn draw(&mut self, ctx: &mut DrawContext<'_>) {
        let system = &mut *self
            .common
            .system
            .as_ref()
            .expect("cannot draw when unmounted")
            .0
            .borrow_mut();

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
        }

        if let Some(pixmap) = &self.pixmap {
            ctx.draw_pixmap(Point::default(), pixmap.as_ref());
        }
    }

    fn mouse_input(&mut self, event: &mut crate::event::MouseInputEvent<'_>) {
        let mut system = self
            .common
            .system
            .as_ref()
            .expect("cannot handle event when unmounted")
            .0
            .borrow_mut();

        if event.state == ElementState::Pressed {
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

    fn cursor_moved(&mut self, event: &mut CursorMovedEvent<'_>) {
        let mut system = self
            .common
            .system
            .as_ref()
            .expect("cannot handle event when unmounted")
            .0
            .borrow_mut();

        if event.pressed_mouse_buttons.contains(&MouseButton::Left) {
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
    }

    fn keyboard_input(&mut self, event: &mut KeyboardInputEvent) {
        if event.input.state == ElementState::Released {
            return;
        }

        let mut system = self
            .common
            .system
            .as_ref()
            .expect("cannot handle event when unmounted")
            .0
            .borrow_mut();

        // println!("ok2 {:?}", event.char);
        if let Some(editor) = &mut self.editor {
            let Some(keycode) =  event.input.virtual_keycode else { return };
            // TODO: different commands for macOS?
            let action = match keycode {
                // TODO: scroll lock?
                VirtualKeyCode::Escape => {
                    // TODO: cosmic-text for some reason suggests to clear selection on Escape?
                    return;
                }
                VirtualKeyCode::Insert => todo!(),
                VirtualKeyCode::Home => Action::Home,
                VirtualKeyCode::Delete => Action::Delete,
                VirtualKeyCode::End => Action::End,
                VirtualKeyCode::PageDown => Action::PageDown,
                VirtualKeyCode::PageUp => Action::PageUp,
                VirtualKeyCode::Left => {
                    if event.modifiers.shift() && editor.select_opt().is_none() {
                        editor.set_select_opt(Some(editor.cursor()));
                    }
                    if !event.modifiers.shift() && editor.select_opt().is_some() {
                        editor.set_select_opt(None);
                    }
                    Action::Left
                }
                VirtualKeyCode::Up => Action::Up,
                VirtualKeyCode::Right => Action::Right,
                VirtualKeyCode::Down => Action::Down,
                VirtualKeyCode::Back => Action::Backspace,
                VirtualKeyCode::Return => Action::Enter,
                VirtualKeyCode::Caret => {
                    // TODO: what's that?
                    return;
                }
                VirtualKeyCode::Copy | VirtualKeyCode::Cut | VirtualKeyCode::Paste => {
                    // TODO
                    return;
                }
                _ => {
                    return;
                }
            };
            // println!(
            //     "ok2.2 selection: {:?}, cursor: {:?}",
            //     editor.select_opt(),
            //     editor.cursor()
            // );
            editor.action(&mut system.font_system, action);
            println!("###");
            for line in &editor.buffer().lines {
                println!("ok1 {:?}", line.text());
                println!("ok2 {:?}", line.text_without_ime());
            }
            //editor.buffer_mut().set_redraw(true);
        }
    }

    fn received_character(&mut self, event: &mut ReceivedCharacterEvent) {
        let mut system = self
            .common
            .system
            .as_ref()
            .expect("cannot handle event when unmounted")
            .0
            .borrow_mut();

        if let Some(editor) = &mut self.editor {
            // TODO: replace line breaks to avoid multiple lines in buffer
            editor.action(&mut system.font_system, Action::Insert(event.char));
            for line in &editor.buffer().lines {
                println!("ok3 {:?}", line.text());
            }
            println!("###");
            for line in &editor.buffer().lines {
                println!("ok1 {:?}", line.text());
                println!("ok2 {:?}", line.text_without_ime());
            }
        }
    }

    fn ime(&mut self, event: &mut ImeEvent) {
        let mut system = self
            .common
            .system
            .as_ref()
            .expect("cannot handle event when unmounted")
            .0
            .borrow_mut();

        if let Some(editor) = &mut self.editor {
            match event.0.clone() {
                Ime::Enabled => {}
                Ime::Preedit(pretext, cursor) => {
                    // TODO: can pretext have line breaks?
                    editor.action(
                        &mut system.font_system,
                        Action::ImeSetPretext { pretext, cursor },
                    );
                }
                Ime::Commit(string) => {
                    editor.action(&mut system.font_system, Action::ImeCommit(string));
                }
                Ime::Disabled => {}
            }
            println!("###");
            for line in &editor.buffer().lines {
                println!("ok1 {:?}", line.text());
                println!("ok2 {:?}", line.text_without_ime());
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
