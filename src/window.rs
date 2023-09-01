use std::{cell::RefCell, collections::HashSet, num::NonZeroU32, rc::Rc};

use tiny_skia::Pixmap;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, ModifiersState, MouseButton, VirtualKeyCode, WindowEvent},
};

use crate::{
    draw::DrawEvent,
    event::{
        CursorMovedEvent, ImeEvent, KeyboardInputEvent, MouseInputEvent, ReceivedCharacterEvent,
    },
    types::{Point, Rect, Size},
    widgets::{
        get_widget_by_address_mut, mount, unmount, MountPoint, RawWidgetId, Widget, WidgetAddress, WidgetExt,
    },
    SharedSystemData,
};

pub struct SharedWindowDataInner {
    pub widget_tree_changed: bool,
    pub cursor_position: Option<Point>,
    pub cursor_entered: bool,
    pub modifiers_state: ModifiersState,
    pub pressed_mouse_buttons: HashSet<MouseButton>,
}

#[derive(Clone)]
pub struct SharedWindowData(pub Rc<RefCell<SharedWindowDataInner>>);

pub struct Window {
    pub inner: winit::window::Window,
    pub softbuffer_context: softbuffer::Context,
    pub surface: softbuffer::Surface,
    pub widget: Option<Box<dyn Widget>>,
    shared_system_data: SharedSystemData,
    shared_window_data: SharedWindowData,

    pub focusable_widgets: Vec<RawWidgetId>,
    pub focused_widget: Option<RawWidgetId>,
}

impl Window {
    pub fn new(
        inner: winit::window::Window,
        shared_system_data: SharedSystemData,
        mut widget: Option<Box<dyn Widget>>,
    ) -> Self {
        inner.set_ime_allowed(true);
        inner.set_ime_position(PhysicalPosition::new(10, 10));
        let softbuffer_context = unsafe { softbuffer::Context::new(&inner) }.unwrap();
        let shared_window_data = SharedWindowData(Rc::new(RefCell::new(SharedWindowDataInner {
            widget_tree_changed: false,
            cursor_position: None,
            cursor_entered: false,
            modifiers_state: ModifiersState::default(),
            pressed_mouse_buttons: HashSet::new(),
        })));
        if let Some(widget) = &mut widget {
            let address = WidgetAddress::window_root(inner.id()).join(widget.common().id);
            mount(
                widget.as_mut(),
                MountPoint {
                    address,
                    system: shared_system_data.clone(),
                    window: shared_window_data.clone(),
                },
            );
        }
        let mut w = Self {
            surface: unsafe { softbuffer::Surface::new(&softbuffer_context, &inner) }.unwrap(),
            softbuffer_context,
            inner,
            widget,
            shared_system_data,
            shared_window_data,
            focusable_widgets: Vec::new(),
            focused_widget: None,
        };
        w.refresh_focusable_widgets();
        w
    }

    pub fn handle_event(&mut self, _ctx: &mut WindowEventContext, event: Event<()>) {
        self.check_widget_tree_change_flag();
        match event {
            Event::RedrawRequested(_) => {
                // Grab the window's client area dimensions
                let (width, height) = {
                    let size = self.inner.inner_size();
                    (size.width, size.height)
                };

                // Resize surface if needed
                self.surface
                    .resize(
                        NonZeroU32::new(width).unwrap(),
                        NonZeroU32::new(height).unwrap(),
                    )
                    .unwrap();

                // Draw something in the window
                let mut buffer = self.surface.buffer_mut().unwrap();

                let pixmap = Pixmap::new(width, height).unwrap();
                let pixmap = Rc::new(RefCell::new(pixmap));
                let draw_event = DrawEvent {
                    rect: Rect {
                        top_left: Point::default(),
                        size: Size {
                            x: width as i32,
                            y: height as i32,
                        },
                    },
                    pixmap: Rc::clone(&pixmap),
                };
                // TODO: option to turn off background, set style
                draw_event.pixmap
                    .borrow_mut()
                    .fill(self.shared_system_data.0.borrow().palette.background);
                if let Some(widget) = &mut self.widget {
                    widget.dispatch(draw_event.into());
                }

                buffer.copy_from_slice(bytemuck::cast_slice(pixmap.borrow().data()));

                // tiny-skia uses an RGBA format, while softbuffer uses XRGB. To convert, we need to
                // iterate over the pixels and shift the pixels over.
                buffer.iter_mut().for_each(|pixel| {
                    let [r, g, b, _] = pixel.to_ne_bytes();
                    *pixel = (b as u32) | ((g as u32) << 8) | ((r as u32) << 16);
                });

                //redraw(&mut buffer, width as usize, height as usize, flag);
                buffer.present().unwrap();
            }
            Event::WindowEvent { event, .. } => {
                if matches!(
                    event,
                    WindowEvent::Ime(_)
                        | WindowEvent::ReceivedCharacter(_)
                        | WindowEvent::KeyboardInput { .. }
                ) {
                    println!("{event:?}");
                }
                match event {
                    // TODO: should use device id?
                    WindowEvent::CursorEntered { .. } => {
                        self.shared_window_data.0.borrow_mut().cursor_entered = true;
                    }
                    WindowEvent::CursorLeft { .. } => {
                        self.shared_window_data.0.borrow_mut().cursor_entered = false;
                        self.shared_window_data.0.borrow_mut().cursor_position = None;
                    }
                    WindowEvent::CursorMoved {
                        position,
                        device_id,
                        ..
                    } => {
                        let pos = Point {
                            // TODO: is round() fine?
                            x: position.x.round() as i32,
                            y: position.y.round() as i32,
                        };
                        self.shared_window_data.0.borrow_mut().cursor_position = Some(pos);
                        if let Some(widget) = &mut self.widget {
                            widget.dispatch(CursorMovedEvent { device_id, pos }.into());
                            self.inner.request_redraw(); // TODO: smarter redraw
                        }
                    }
                    WindowEvent::ModifiersChanged(state) => {
                        self.shared_window_data.0.borrow_mut().modifiers_state = state;
                    }
                    WindowEvent::MouseInput {
                        device_id,
                        state,
                        button,
                        ..
                    } => {
                        match state {
                            ElementState::Pressed => {
                                self.shared_window_data
                                    .0
                                    .borrow_mut()
                                    .pressed_mouse_buttons
                                    .insert(button);
                            }
                            ElementState::Released => {
                                self.shared_window_data
                                    .0
                                    .borrow_mut()
                                    .pressed_mouse_buttons
                                    .remove(&button);
                            }
                        }
                        let cursor_position = self.shared_window_data.0.borrow().cursor_position;
                        if let Some(pos) = cursor_position {
                            if let Some(widget) = &mut self.widget {
                                widget.dispatch(MouseInputEvent {
                                    device_id,
                                    state,
                                    button,
                                    pos,
                                }.into());
                                self.inner.request_redraw(); // TODO: smarter redraw
                            }
                        } else {
                            println!("warning: no cursor position in mouse input handler");
                        }
                    }
                    WindowEvent::KeyboardInput {
                        input,
                        device_id,
                        is_synthetic,
                    } => {
                        // TODO: deduplicate with ReceivedCharacter
                        if let Some(root_widget) = &mut self.widget {
                            if let Some(focused_widget) = self.focused_widget {
                                let address: Option<WidgetAddress> = self
                                    .shared_system_data
                                    .0
                                    .borrow()
                                    .address_book
                                    .get(&focused_widget)
                                    .cloned();
                                if let Some(address) = address {
                                    if let Ok(widget) =
                                        get_widget_by_address_mut(root_widget.as_mut(), &address)
                                    {
                                        widget.dispatch(KeyboardInputEvent {
                                            device_id,
                                            input,
                                            is_synthetic,
                                        }.into());
                                        self.inner.request_redraw(); // TODO: smarter redraw
                                    }
                                }
                            }
                        }

                        // TODO: only if event is not accepted by a widget
                        if input.state == ElementState::Pressed {
                            if let Some(virtual_keycode) = input.virtual_keycode {
                                if virtual_keycode == VirtualKeyCode::Tab {
                                    if self.shared_window_data.0.borrow().modifiers_state.shift() {
                                        self.move_keyboard_focus(-1);
                                    } else {
                                        self.move_keyboard_focus(1);
                                    }
                                }
                            }
                        }
                    }
                    WindowEvent::ReceivedCharacter(char) => {
                        if let Some(root_widget) = &mut self.widget {
                            if let Some(focused_widget) = self.focused_widget {
                                let address: Option<WidgetAddress> = self
                                    .shared_system_data
                                    .0
                                    .borrow()
                                    .address_book
                                    .get(&focused_widget)
                                    .cloned();
                                if let Some(address) = address {
                                    if let Ok(widget) =
                                        get_widget_by_address_mut(root_widget.as_mut(), &address)
                                    {
                                        widget.dispatch(ReceivedCharacterEvent {
                                            char,
                                        }.into());
                                        self.inner.request_redraw(); // TODO: smarter redraw
                                    }
                                }
                            }
                        }
                    }
                    WindowEvent::Ime(ime) => {
                        // TODO: deduplicate with ReceivedCharacter
                        if let Some(root_widget) = &mut self.widget {
                            if let Some(focused_widget) = self.focused_widget {
                                let address: Option<WidgetAddress> = self
                                    .shared_system_data
                                    .0
                                    .borrow()
                                    .address_book
                                    .get(&focused_widget)
                                    .cloned();
                                if let Some(address) = address {
                                    if let Ok(widget) =
                                        get_widget_by_address_mut(root_widget.as_mut(), &address)
                                    {
                                        widget.dispatch(ImeEvent(ime).into());
                                        self.inner.request_redraw(); // TODO: smarter redraw
                                    }
                                }
                            }
                        }
                        //self.inner.set_ime_position(PhysicalPosition::new(10, 10));
                    }
                    // WindowEvent::Ime(Ime::Preedit(text, cursor)) => {
                    //     //...
                    //     if let Some((start, _end)) = cursor {
                    //         println!("{}|{}", &text[..start], &text[start..]);
                    //     }
                    // }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn move_keyboard_focus(&mut self, direction: i32) {
        if self.focusable_widgets.is_empty() {
            return;
        }
        if let Some(focused_widget) = self.focused_widget {
            if let Some(index) = self
                .focusable_widgets
                .iter()
                .position(|i| *i == focused_widget)
            {
                let new_index =
                    (index as i32 + direction).rem_euclid(self.focusable_widgets.len() as i32);
                self.focused_widget = Some(self.focusable_widgets[new_index as usize]);
            } else {
                println!("warn: focused widget is unknown");
            }
        } else {
            self.focused_widget = Some(self.focusable_widgets[0]);
        }
        println!("new focused: {:?}", self.focused_widget);
    }

    pub fn set_widget(&mut self, mut widget: Option<Box<dyn Widget>>) {
        if let Some(old_widget) = &mut self.widget {
            unmount(old_widget.as_mut());
        }
        if let Some(widget) = &mut widget {
            let address = WidgetAddress::window_root(self.inner.id()).join(widget.common().id);
            mount(
                widget.as_mut(),
                MountPoint {
                    address,
                    system: self.shared_system_data.clone(),
                    window: self.shared_window_data.clone(),
                },
            );
        }
        self.widget = widget;
        self.refresh_focusable_widgets();
    }

    fn check_widget_tree_change_flag(&mut self) {
        {
            let mut shared = self.shared_window_data.0.borrow_mut();
            if !shared.widget_tree_changed {
                return;
            }
            shared.widget_tree_changed = false;
        }
        self.refresh_focusable_widgets();
    }

    fn refresh_focusable_widgets(&mut self) {
        self.focusable_widgets.clear();
        if let Some(widget) = &mut self.widget {
            populate_focusable_widgets(widget.as_mut(), &mut self.focusable_widgets);
        }
        if let Some(focused_widget) = &self.focused_widget {
            if !self.focusable_widgets.contains(focused_widget) {
                self.focused_widget = None;
            }
        }
        if self.focused_widget.is_none() {
            self.focused_widget = self.focusable_widgets.get(0).copied();
        }
        println!("new focused after refresh: {:?}", self.focused_widget);
    }
}

// TODO: not mut
fn populate_focusable_widgets(widget: &mut dyn Widget, output: &mut Vec<RawWidgetId>) {
    if widget.common().is_focusable {
        output.push(widget.common().id);
    }
    for child in widget.children_mut() {
        populate_focusable_widgets(child.as_mut(), output);
    }
}

pub struct WindowEventContext {}
