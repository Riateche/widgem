use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    num::NonZeroU32,
    rc::Rc,
};

use derive_more::From;
use tiny_skia::Pixmap;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, Event, Ime, MouseButton, WindowEvent},
    keyboard::{Key, ModifiersState},
};

use crate::{
    draw::DrawEvent,
    event::{
        CursorMovedEvent, FocusInEvent, FocusOutEvent, FocusReason, GeometryChangedEvent, ImeEvent,
        KeyboardInputEvent, MountEvent, MouseInputEvent, UnmountEvent,
    },
    system::with_system,
    types::{Point, Rect, Size},
    widgets::{
        get_widget_by_id_mut, Geometry, MountPoint, RawWidgetId, Widget, WidgetAddress, WidgetExt,
    },
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
    pub root_widget: Option<Box<dyn Widget>>,
    shared_window_data: SharedWindowData,

    pub focusable_widgets: Vec<RawWidgetId>,
    pub focused_widget: Option<RawWidgetId>,
    pub mouse_grabber_widget: Option<RawWidgetId>,
    ime_cursor_area: Rect,
}

impl Window {
    pub fn new(inner: winit::window::Window, mut widget: Option<Box<dyn Widget>>) -> Self {
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
            widget.dispatch(
                MountEvent(MountPoint {
                    address,
                    window: shared_window_data.clone(),
                })
                .into(),
            );
        }
        let mut w = Self {
            surface: unsafe { softbuffer::Surface::new(&softbuffer_context, &inner) }.unwrap(),
            softbuffer_context,
            inner,
            root_widget: widget,
            shared_window_data,
            focusable_widgets: Vec::new(),
            focused_widget: None,
            mouse_grabber_widget: None,
            ime_cursor_area: Rect::default(),
        };
        w.widget_tree_changed();
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
                let color = with_system(|system| system.palette.background);
                draw_event.pixmap.borrow_mut().fill(color);
                if let Some(widget) = &mut self.root_widget {
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
                if matches!(event, WindowEvent::Ime(_)) {
                    println!("{event:?}");
                }
                if matches!(event, WindowEvent::KeyboardInput { .. }) {
                    println!("keyborard input event: {event:?}");
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
                        let pos_in_window = Point {
                            // TODO: is round() fine?
                            x: position.x.round() as i32,
                            y: position.y.round() as i32,
                        };
                        self.shared_window_data.0.borrow_mut().cursor_position =
                            Some(pos_in_window);
                        if let Some(root_widget) = &mut self.root_widget {
                            if let Some(mouse_grabber_widget_id) = self.mouse_grabber_widget {
                                if let Ok(mouse_grabber_widget) = get_widget_by_id_mut(
                                    root_widget.as_mut(),
                                    mouse_grabber_widget_id,
                                ) {
                                    if let Some(geometry) = mouse_grabber_widget.common().geometry {
                                        let pos_in_widget =
                                            pos_in_window - geometry.rect_in_window.top_left;
                                        mouse_grabber_widget.dispatch(
                                            CursorMovedEvent {
                                                device_id,
                                                pos: pos_in_widget,
                                            }
                                            .into(),
                                        );
                                    }
                                }
                            } else {
                                root_widget.dispatch(
                                    CursorMovedEvent {
                                        device_id,
                                        pos: pos_in_window,
                                    }
                                    .into(),
                                );
                            }
                        }
                        self.inner.request_redraw(); // TODO: smarter redraw
                    }
                    WindowEvent::ModifiersChanged(modifiers) => {
                        self.shared_window_data.0.borrow_mut().modifiers_state = modifiers.state();
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
                        if let Some(pos_in_window) = cursor_position {
                            if let Some(root_widget) = &mut self.root_widget {
                                let accepted_by = Rc::new(Cell::new(None));
                                if let Some(mouse_grabber_widget_id) = self.mouse_grabber_widget {
                                    if let Ok(mouse_grabber_widget) = get_widget_by_id_mut(
                                        root_widget.as_mut(),
                                        mouse_grabber_widget_id,
                                    ) {
                                        if let Some(geometry) =
                                            mouse_grabber_widget.common().geometry
                                        {
                                            let pos_in_widget =
                                                pos_in_window - geometry.rect_in_window.top_left;
                                            mouse_grabber_widget.dispatch(
                                                MouseInputEvent {
                                                    device_id,
                                                    state,
                                                    button,
                                                    pos: pos_in_widget,
                                                    accepted_by: Rc::clone(&accepted_by),
                                                }
                                                .into(),
                                            );
                                        }
                                    }
                                    if self
                                        .shared_window_data
                                        .0
                                        .borrow_mut()
                                        .pressed_mouse_buttons
                                        .is_empty()
                                    {
                                        self.mouse_grabber_widget = None;
                                    }
                                } else {
                                    root_widget.dispatch(
                                        MouseInputEvent {
                                            device_id,
                                            state,
                                            button,
                                            pos: pos_in_window,
                                            accepted_by: Rc::clone(&accepted_by),
                                        }
                                        .into(),
                                    );
                                    if state == ElementState::Pressed {
                                        if let Some(accepted_by_widget_id) = accepted_by.get() {
                                            self.mouse_grabber_widget = Some(accepted_by_widget_id);
                                        }
                                    }
                                }

                                self.inner.request_redraw(); // TODO: smarter redraw
                            }
                        } else {
                            println!("warning: no cursor position in mouse input handler");
                        }
                    }
                    WindowEvent::KeyboardInput {
                        device_id,
                        is_synthetic,
                        event,
                    } => {
                        // TODO: deduplicate with ReceivedCharacter
                        if let Some(root_widget) = &mut self.root_widget {
                            if let Some(focused_widget) = self.focused_widget {
                                if let Ok(widget) =
                                    get_widget_by_id_mut(root_widget.as_mut(), focused_widget)
                                {
                                    widget.dispatch(
                                        KeyboardInputEvent {
                                            device_id,
                                            event: event.clone(),
                                            is_synthetic,
                                        }
                                        .into(),
                                    );
                                    self.inner.request_redraw(); // TODO: smarter redraw
                                }
                            }
                        }

                        // TODO: only if event is not accepted by a widget
                        if event.state == ElementState::Pressed {
                            let logical_key = event.logical_key;
                            if logical_key == Key::Tab {
                                if self
                                    .shared_window_data
                                    .0
                                    .borrow()
                                    .modifiers_state
                                    .shift_key()
                                {
                                    self.move_keyboard_focus(-1);
                                } else {
                                    self.move_keyboard_focus(1);
                                }
                            }
                        }
                    }
                    WindowEvent::Ime(ime) => {
                        if let Ime::Enabled = &ime {
                            println!("reset ime position {:?}", self.ime_cursor_area);
                            self.inner.set_ime_cursor_area(
                                PhysicalPosition::new(
                                    self.ime_cursor_area.top_left.x,
                                    self.ime_cursor_area.top_left.y,
                                ),
                                PhysicalSize::new(
                                    self.ime_cursor_area.size.x,
                                    self.ime_cursor_area.size.y,
                                ),
                            );
                        }
                        // TODO: deduplicate with ReceivedCharacter
                        if let Some(root_widget) = &mut self.root_widget {
                            if let Some(focused_widget) = self.focused_widget {
                                if let Ok(widget) =
                                    get_widget_by_id_mut(root_widget.as_mut(), focused_widget)
                                {
                                    widget.dispatch(ImeEvent(ime).into());
                                    self.inner.request_redraw(); // TODO: smarter redraw
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
        let reason = FocusReason::Tab;
        if let Some(focused_widget) = self.focused_widget {
            if let Some(index) = self
                .focusable_widgets
                .iter()
                .position(|i| *i == focused_widget)
            {
                let new_index =
                    (index as i32 + direction).rem_euclid(self.focusable_widgets.len() as i32);
                self.set_focus(self.focusable_widgets[new_index as usize], reason);
            } else {
                println!("warn: focused widget is unknown");
                self.unset_focus();
            }
        } else {
            println!("warn: no focused widget");
        }
        println!("new focused: {:?}", self.focused_widget);
        self.check_auto_focus();
    }

    pub fn set_widget(&mut self, mut widget: Option<Box<dyn Widget>>) {
        if let Some(old_widget) = &mut self.root_widget {
            old_widget.dispatch(UnmountEvent.into());
        }
        if let Some(widget) = &mut widget {
            let address = WidgetAddress::window_root(self.inner.id()).join(widget.common().id);
            widget.dispatch(
                MountEvent(MountPoint {
                    address,
                    window: self.shared_window_data.clone(),
                })
                .into(),
            );
        }
        self.root_widget = widget;
        self.widget_tree_changed();
    }

    fn check_widget_tree_change_flag(&mut self) {
        {
            let mut shared = self.shared_window_data.0.borrow_mut();
            if !shared.widget_tree_changed {
                return;
            }
            shared.widget_tree_changed = false;
        }
        self.widget_tree_changed();
    }

    fn widget_tree_changed(&mut self) {
        self.refresh_focusable_widgets();
        self.layout();
    }

    fn refresh_focusable_widgets(&mut self) {
        self.focusable_widgets.clear();
        if let Some(widget) = &mut self.root_widget {
            populate_focusable_widgets(widget.as_mut(), &mut self.focusable_widgets);
        }
        if let Some(focused_widget) = &self.focused_widget {
            if !self.focusable_widgets.contains(focused_widget) {
                self.unset_focus();
            }
        }
        self.check_auto_focus();
        println!("new focused after refresh: {:?}", self.focused_widget);
    }

    fn check_auto_focus(&mut self) {
        if self.focused_widget.is_none() {
            if let Some(&id) = self.focusable_widgets.get(0) {
                self.set_focus(id, FocusReason::Auto);
            }
        }
    }

    fn set_focus(&mut self, widget_id: RawWidgetId, reason: FocusReason) {
        let Some(root_widget) = &mut self.root_widget else {
            println!("warn: set_focus: no root widget");
            return;
        };
        if let Ok(widget) = get_widget_by_id_mut(root_widget.as_mut(), widget_id) {
            if !widget.common().is_focusable {
                println!("warn: cannot focus widget that is not focusable");
                return;
            }
            self.inner.set_ime_allowed(widget.common().enable_ime);
        } else {
            println!("warn: set_focus: widget not found");
        }

        if let Some(old_widget_id) = self.focused_widget.take() {
            if let Ok(old_widget) = get_widget_by_id_mut(root_widget.as_mut(), old_widget_id) {
                old_widget.dispatch(FocusOutEvent.into());
            }
        }

        if let Ok(widget) = get_widget_by_id_mut(root_widget.as_mut(), widget_id) {
            widget.dispatch(FocusInEvent { reason }.into());
            self.focused_widget = Some(widget_id);
        } else {
            println!("warn: set_focus: widget not found on second pass");
        }
        self.inner.request_redraw(); // TODO: smarter redraw
    }

    fn unset_focus(&mut self) {
        self.focused_widget = None;
        self.inner.set_ime_allowed(false);
    }

    fn layout(&mut self) {
        if let Some(root) = &mut self.root_widget {
            // TODO: only on insert or resize
            root.dispatch(
                GeometryChangedEvent {
                    new_geometry: Some(Geometry {
                        rect_in_window: Rect {
                            top_left: Point::default(),
                            size: Size {
                                x: self.inner.inner_size().width as i32,
                                y: self.inner.inner_size().height as i32,
                            },
                        },
                    }),
                }
                .into(),
            );
            root.layout();
        }
    }

    pub fn handle_request(&mut self, _ctx: &mut WindowEventContext, request: WindowRequest) {
        match request {
            WindowRequest::SetFocus(request) => {
                self.set_focus(request.widget_id, request.reason);
            }
            WindowRequest::SetImeCursorArea(request) => {
                println!("set new ime position {:?}", request.0);
                self.inner.set_ime_cursor_area(
                    PhysicalPosition::new(request.0.top_left.x, request.0.top_left.y),
                    PhysicalSize::new(request.0.size.x, request.0.size.y),
                ); //TODO: actual size
                self.ime_cursor_area = request.0;
            }
        }
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

#[derive(Debug, From)]
pub enum WindowRequest {
    SetFocus(SetFocusRequest),
    SetImeCursorArea(SetImeCursorAreaRequest),
}

#[derive(Debug)]
pub struct SetFocusRequest {
    pub widget_id: RawWidgetId,
    pub reason: FocusReason,
}

#[derive(Debug)]
pub struct SetImeCursorAreaRequest(pub Rect);
