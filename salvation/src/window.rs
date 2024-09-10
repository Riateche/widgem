use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    mem,
    num::NonZeroU32,
    rc::Rc,
    time::{Duration, Instant},
};

use accesskit::ActionRequest;
use derivative::Derivative;
use derive_more::From;
use log::{trace, warn};
use tiny_skia::Pixmap;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, Ime, MouseButton, WindowEvent},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{CursorIcon, WindowAttributes, WindowId},
};

use crate::{
    accessible::AccessibleNodes,
    draw::DrawEvent,
    event::{
        AccessibleActionEvent, FocusInEvent, FocusOutEvent, FocusReason, ImeEvent,
        KeyboardInputEvent, LayoutEvent, MouseInputEvent, MouseLeaveEvent, MouseMoveEvent,
        WindowFocusChangeEvent,
    },
    event_loop::{with_active_event_loop, UserEvent},
    system::{address, with_system},
    types::{Point, Rect, Size},
    widgets::{
        get_widget_by_id_mut, invalidate_size_hint_cache, RawWidgetId, Widget, WidgetAddress,
        WidgetExt,
    },
};

// TODO: get system setting
const DOUBLE_CLICK_TIMEOUT: Duration = Duration::from_millis(300);

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SharedWindowDataInner {
    pub id: WindowId,
    pub root_widget_id: RawWidgetId,
    pub cursor_position: Option<Point>,
    pub cursor_entered: bool,
    pub modifiers_state: ModifiersState,
    pub pressed_mouse_buttons: HashSet<MouseButton>,
    pub is_window_focused: bool,
    pub accessible_nodes: AccessibleNodes,
    pub mouse_entered_widgets: Vec<(Rect, RawWidgetId)>,
    pub pending_size_hint_invalidations: Vec<WidgetAddress>,
    pub pixmap: Rc<RefCell<Pixmap>>,
    // Drop order must be maintained as
    // `surface` -> `softbuffer_context` -> `winit_window`
    // to maintain safety requirements.
    #[derivative(Debug = "ignore")]
    pub surface: softbuffer::Surface<Rc<winit::window::Window>, Rc<winit::window::Window>>,
    #[derivative(Debug = "ignore")]
    pub softbuffer_context: softbuffer::Context<Rc<winit::window::Window>>,
    #[derivative(Debug = "ignore")]
    pub accesskit_adapter: accesskit_winit::Adapter,
    pub winit_window: Rc<winit::window::Window>,
    pub ime_allowed: bool,
    pub ime_cursor_area: Rect,

    pub pending_redraw: bool,

    pub focusable_widgets: Vec<(WidgetAddress, RawWidgetId)>,
    pub focusable_widgets_changed: bool,

    pub focused_widget: Option<(WidgetAddress, RawWidgetId)>,
    pub mouse_grabber_widget: Option<RawWidgetId>,
    pub num_clicks: u32,
    pub last_click_button: Option<MouseButton>,
    pub last_click_instant: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct SharedWindowData(pub Rc<RefCell<SharedWindowDataInner>>);

impl SharedWindowData {
    pub fn pop_mouse_entered_widget(&self) -> Option<RawWidgetId> {
        let this = &mut *self.0.borrow_mut();
        let pos = this.cursor_position;
        let list = &mut this.mouse_entered_widgets;
        let index = list.iter().position(|(rect, id)| {
            pos.map_or(true, |pos| !rect.contains(pos)) && Some(*id) != this.mouse_grabber_widget
        })?;
        Some(list.remove(index).1)
    }

    pub fn set_ime_cursor_area(&self, rect: Rect) {
        let mut this = self.0.borrow_mut();
        if this.ime_cursor_area != rect {
            this.winit_window.set_ime_cursor_area(
                PhysicalPosition::new(rect.top_left.x, rect.top_left.y),
                PhysicalSize::new(rect.size.x, rect.size.y),
            ); //TODO: actual size
            this.ime_cursor_area = rect;
        }
    }

    // TODO: should there be a proper way to do this in winit?
    pub fn cancel_ime_preedit(&self) {
        let this = self.0.borrow();
        if this.ime_allowed {
            this.winit_window.set_ime_allowed(false);
            this.winit_window.set_ime_allowed(true);
        }
    }

    pub fn request_redraw(&self) {
        let mut this = self.0.borrow_mut();
        if !this.pending_redraw {
            this.pending_redraw = true;
            this.winit_window.request_redraw();
        }
    }

    pub fn add_focusable_widget(&self, addr: WidgetAddress, id: RawWidgetId) {
        let mut this = self.0.borrow_mut();
        let pair = (addr, id);
        if let Err(index) = this.focusable_widgets.binary_search(&pair) {
            this.focusable_widgets.insert(index, pair);
            this.focusable_widgets_changed = true;
        }
    }

    pub fn remove_focusable_widget(&self, addr: WidgetAddress, id: RawWidgetId) {
        let mut this = self.0.borrow_mut();
        let pair = (addr, id);
        if let Ok(index) = this.focusable_widgets.binary_search(&pair) {
            this.focusable_widgets.remove(index);
            this.focusable_widgets_changed = true;
        }
    }

    fn move_keyboard_focus(
        &self,
        focused_widget: &(WidgetAddress, RawWidgetId),
        direction: i32,
    ) -> Option<(WidgetAddress, RawWidgetId)> {
        let this = self.0.borrow();
        if this.focusable_widgets.is_empty() {
            return None;
        }
        if let Ok(index) = this.focusable_widgets.binary_search(focused_widget) {
            let new_index =
                (index as i32 + direction).rem_euclid(this.focusable_widgets.len() as i32);
            Some(this.focusable_widgets[new_index as usize].clone())
        } else {
            warn!("focused widget is unknown");
            this.focusable_widgets.first().cloned()
        }
    }
}

// TODO: rename to something like WindowSystemData and hide
#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub shared_window_data: SharedWindowData,
    pub root_widget_id: RawWidgetId,
}

pub struct WindowWithWidget<'a> {
    pub id: WindowId,
    pub shared_window_data: SharedWindowData,
    pub root_widget: &'a mut dyn Widget,
}

pub fn create_window(attrs: WindowAttributes, widget: &mut dyn Widget) -> SharedWindowData {
    // println!("create_window OK!");
    let w = Window::new(attrs, widget);
    let data = w.shared_window_data.clone();
    with_system(|system| {
        system.windows.insert(w.id, w);
    });
    data
}

// Extra size to avoid visual artifacts when resizing the window.
// Must be > 0 to avoid panic on pixmap creation.
const EXTRA_SURFACE_SIZE: u32 = 50;

impl Window {
    pub fn new(mut attrs: WindowAttributes, widget: &mut dyn Widget) -> Self {
        // TODO: propagate style without mounting?
        let size_hints_x = widget.size_hints_x();
        // TODO: adjust size_x for screen size
        let size_hints_y = widget.size_hints_y(size_hints_x.preferred);
        attrs = attrs
            .with_inner_size(PhysicalSize::new(
                size_hints_x.preferred,
                size_hints_y.preferred,
            ))
            .with_min_inner_size(PhysicalSize::new(size_hints_x.min, size_hints_y.min));
        let winit_window = Rc::new(with_active_event_loop(|event_loop| {
            event_loop.create_window(attrs).unwrap()
        }));
        let softbuffer_context = softbuffer::Context::new(winit_window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&softbuffer_context, winit_window.clone()).unwrap();
        let window_id = winit_window.id();
        let accesskit_adapter = accesskit_winit::Adapter::with_event_loop_proxy(
            &winit_window,
            with_system(|system| system.event_loop_proxy.clone()),
        );
        let inner_size = winit_window.inner_size();

        let shared_window_data = SharedWindowData(Rc::new(RefCell::new(SharedWindowDataInner {
            id: winit_window.id(),
            root_widget_id: widget.common().id,
            cursor_position: None,
            cursor_entered: false,
            modifiers_state: ModifiersState::default(),
            pressed_mouse_buttons: HashSet::new(),
            is_window_focused: false,
            accessible_nodes: AccessibleNodes::new(),
            mouse_entered_widgets: Vec::new(),
            pending_size_hint_invalidations: Vec::new(),
            pixmap: Rc::new(RefCell::new(
                Pixmap::new(
                    inner_size.width + EXTRA_SURFACE_SIZE,
                    inner_size.height + EXTRA_SURFACE_SIZE,
                )
                .unwrap(),
            )),
            surface,
            softbuffer_context,
            accesskit_adapter,
            winit_window,
            ime_allowed: false,
            ime_cursor_area: Rect::default(),
            pending_redraw: false,
            focusable_widgets: Vec::new(),
            focusable_widgets_changed: false,
            focused_widget: None,
            mouse_grabber_widget: None,
            num_clicks: 0,
            last_click_button: None,
            last_click_instant: None,
        })));

        let mut scope = widget.common().scope.clone();
        scope.window = Some(shared_window_data.clone());
        widget.set_scope(scope);

        // Window must be hidden until we initialize accesskit
        shared_window_data.0.borrow().winit_window.set_visible(true);
        // For some reason it's necessary to request redraw again after initializing accesskit on Windows.
        shared_window_data.0.borrow().winit_window.request_redraw();

        let w = Self {
            id: window_id,
            root_widget_id: widget.common().id,
            shared_window_data,
        };
        w.clone().with_root(widget).after_widget_activity();

        // {
        //     let pixmap = Pixmap::decode_png(include_bytes!("../assets/icon.png")).unwrap();
        //     w.shared_window_data
        //         .0
        //         .borrow()
        //         .winit_window
        //         .set_window_icon(Some(
        //             Icon::from_rgba(pixmap.data().to_vec(), pixmap.width(), pixmap.height())
        //                 .unwrap(),
        //         ));
        // }
        w
    }

    pub fn with_root(self, root_widget: &mut dyn Widget) -> WindowWithWidget<'_> {
        WindowWithWidget {
            id: self.id,
            shared_window_data: self.shared_window_data,
            root_widget,
        }
    }
}
impl<'a> WindowWithWidget<'a> {
    pub fn close(&mut self) {
        with_system(|system| {
            let _ = system
                .event_loop_proxy
                .send_event(UserEvent::WindowClosed(self.id));
        });
    }

    fn dispatch_cursor_leave(&mut self) {
        while let Some(id) = self.shared_window_data.pop_mouse_entered_widget() {
            if let Ok(widget) = get_widget_by_id_mut(self.root_widget, id) {
                widget.dispatch(MouseLeaveEvent::new().into());
            }
        }
    }

    pub fn handle_event(&mut self, event: WindowEvent) {
        {
            let data = &mut *self.shared_window_data.0.borrow_mut();
            data.accesskit_adapter
                .process_event(&data.winit_window, &event);
        }

        // println!("{:?} {:?}", self.id, event);
        match event {
            WindowEvent::RedrawRequested => {
                let (width, height) = {
                    let size = &self.shared_window_data.0.borrow().winit_window.inner_size();
                    (
                        size.width + EXTRA_SURFACE_SIZE,
                        size.height + EXTRA_SURFACE_SIZE,
                    )
                };

                self.shared_window_data
                    .0
                    .borrow_mut()
                    .surface
                    .resize(
                        NonZeroU32::new(width).unwrap(),
                        NonZeroU32::new(height).unwrap(),
                    )
                    .unwrap();

                {
                    let data = self.shared_window_data.0.borrow();
                    let mut pixmap = data.pixmap.borrow_mut();
                    if pixmap.width() != width || pixmap.height() != height {
                        *pixmap = Pixmap::new(width, height).unwrap();
                    }
                }

                let pending_redraw = self.shared_window_data.0.borrow().pending_redraw;
                // static X: AtomicU64 = AtomicU64::new(0);
                // println!(
                //     "redraw event {pending_redraw} {}",
                //     X.fetch_add(1, Ordering::Relaxed)
                // );
                if pending_redraw {
                    let draw_event = DrawEvent::new(
                        Rc::clone(&self.shared_window_data.0.borrow().pixmap),
                        Point::default(),
                        Rect {
                            top_left: Point::default(),
                            size: Size {
                                x: width as i32,
                                y: height as i32,
                            },
                        },
                    );
                    // TODO: option to turn off background, set style
                    let color = with_system(|system| system.default_style.background);
                    self.shared_window_data
                        .0
                        .borrow()
                        .pixmap
                        .borrow_mut()
                        .fill(color);
                    self.root_widget.dispatch(draw_event.into());

                    self.shared_window_data.0.borrow_mut().pending_redraw = false;
                }

                {
                    // Draw something in the window
                    let window_data = &mut *self.shared_window_data.0.borrow_mut();
                    let mut buffer = window_data.surface.buffer_mut().unwrap();
                    buffer
                        .copy_from_slice(bytemuck::cast_slice(window_data.pixmap.borrow().data()));

                    // tiny-skia uses an RGBA format, while softbuffer uses XRGB. To convert, we need to
                    // iterate over the pixels and shift the pixels over.
                    buffer.iter_mut().for_each(|pixel| {
                        let [r, g, b, _] = pixel.to_ne_bytes();
                        *pixel = (b as u32) | ((g as u32) << 8) | ((r as u32) << 16);
                    });

                    //redraw(&mut buffer, width as usize, height as usize, flag);
                    buffer.present().unwrap();
                }
            }
            WindowEvent::Resized(_) => {
                self.layout(Vec::new());
            }
            WindowEvent::CloseRequested => {
                // TODO: add option to confirm close or do something else
                self.close();
            }
            // TODO: should use device id?
            WindowEvent::CursorEntered { .. } => {
                self.shared_window_data.0.borrow_mut().cursor_entered = true;
            }
            WindowEvent::CursorLeft { .. } => {
                self.shared_window_data.0.borrow_mut().cursor_entered = false;
                self.shared_window_data.0.borrow_mut().cursor_position = None;
                self.dispatch_cursor_leave();
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
                {
                    let mut shared = self.shared_window_data.0.borrow_mut();
                    if shared.cursor_position != Some(pos_in_window) {
                        shared.cursor_position = Some(pos_in_window);
                    } else {
                        return;
                    }
                }
                self.dispatch_cursor_leave();

                let accepted_by = Rc::new(Cell::new(None));
                let mouse_grabber_widget1 = self.shared_window_data.0.borrow().mouse_grabber_widget;
                if let Some(mouse_grabber_widget_id) = mouse_grabber_widget1 {
                    if let Ok(mouse_grabber_widget) =
                        get_widget_by_id_mut(self.root_widget, mouse_grabber_widget_id)
                    {
                        if let Some(rect_in_window) = mouse_grabber_widget.common().rect_in_window {
                            let pos_in_widget = pos_in_window - rect_in_window.top_left;
                            mouse_grabber_widget.dispatch(
                                MouseMoveEvent::builder()
                                    .device_id(device_id)
                                    .pos(pos_in_widget)
                                    .pos_in_window(pos_in_window)
                                    .accepted_by(accepted_by.clone())
                                    .build()
                                    .into(),
                            );
                        }
                    }
                } else {
                    self.root_widget.dispatch(
                        MouseMoveEvent::builder()
                            .device_id(device_id)
                            .pos(pos_in_window)
                            .pos_in_window(pos_in_window)
                            .accepted_by(accepted_by.clone())
                            .build()
                            .into(),
                    );
                }
                if accepted_by.get().is_none() {
                    self.shared_window_data
                        .0
                        .borrow()
                        .winit_window
                        .set_cursor(CursorIcon::Default);
                }
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
                        let had_recent_click = self
                            .shared_window_data
                            .0
                            .borrow()
                            .last_click_instant
                            .map_or(false, |last| last.elapsed() < DOUBLE_CLICK_TIMEOUT);
                        let same_button =
                            self.shared_window_data.0.borrow().last_click_button == Some(button);
                        if had_recent_click && same_button {
                            self.shared_window_data.0.borrow_mut().num_clicks += 1;
                        } else {
                            self.shared_window_data.0.borrow_mut().num_clicks = 1;
                            self.shared_window_data.0.borrow_mut().last_click_button = Some(button);
                        }
                        self.shared_window_data.0.borrow_mut().last_click_instant =
                            Some(Instant::now());
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
                // println!("click pos {:?}", cursor_position);
                if let Some(pos_in_window) = cursor_position {
                    let accepted_by = Rc::new(Cell::new(None));
                    let mouse_grabber_widget1: Option<RawWidgetId> =
                        self.shared_window_data.0.borrow().mouse_grabber_widget;
                    if let Some(mouse_grabber_widget_id) = mouse_grabber_widget1 {
                        if let Ok(mouse_grabber_widget) =
                            get_widget_by_id_mut(self.root_widget, mouse_grabber_widget_id)
                        {
                            if let Some(rect_in_window) =
                                mouse_grabber_widget.common().rect_in_window
                            {
                                let pos_in_widget = pos_in_window - rect_in_window.top_left;
                                let event = MouseInputEvent::builder()
                                    .device_id(device_id)
                                    .state(state)
                                    .button(button)
                                    .num_clicks(self.shared_window_data.0.borrow().num_clicks)
                                    .pos(pos_in_widget)
                                    .pos_in_window(pos_in_window)
                                    .accepted_by(Rc::clone(&accepted_by))
                                    .build();
                                mouse_grabber_widget.dispatch(event.into());
                            }
                        }
                        let no_buttons1 = self
                            .shared_window_data
                            .0
                            .borrow_mut()
                            .pressed_mouse_buttons
                            .is_empty();
                        if no_buttons1 {
                            self.shared_window_data.0.borrow_mut().mouse_grabber_widget = None;
                            self.dispatch_cursor_leave();
                        }
                    } else {
                        let event = MouseInputEvent::builder()
                            .device_id(device_id)
                            .state(state)
                            .button(button)
                            .num_clicks(self.shared_window_data.0.borrow().num_clicks)
                            .pos(pos_in_window)
                            .pos_in_window(pos_in_window)
                            .accepted_by(Rc::clone(&accepted_by))
                            .build();
                        self.root_widget.dispatch(event.into());

                        if state == ElementState::Pressed {
                            if let Some(accepted_by_widget_id) = accepted_by.get() {
                                self.shared_window_data.0.borrow_mut().mouse_grabber_widget =
                                    Some(accepted_by_widget_id);
                            }
                        }
                    }
                } else {
                    warn!("no cursor position in mouse input handler");
                }
            }
            WindowEvent::KeyboardInput {
                device_id,
                is_synthetic,
                event,
            } => {
                let focused_widget1 = self.shared_window_data.0.borrow().focused_widget.clone();
                if let Some(focused_widget) = focused_widget1 {
                    if let Ok(widget) = get_widget_by_id_mut(self.root_widget, focused_widget.1) {
                        let modifiers = self.shared_window_data.0.borrow().modifiers_state;
                        widget.dispatch(
                            KeyboardInputEvent::builder()
                                .device_id(device_id)
                                .info(event.clone())
                                .is_synthetic(is_synthetic)
                                .modifiers(modifiers)
                                .build()
                                .into(),
                        );
                    }
                }

                // TODO: only if event is not accepted by a widget
                if event.state == ElementState::Pressed {
                    let logical_key = event.logical_key;
                    if logical_key == Key::Named(NamedKey::Tab) {
                        let shift1 = self
                            .shared_window_data
                            .0
                            .borrow()
                            .modifiers_state
                            .shift_key();
                        if shift1 {
                            self.move_keyboard_focus(-1);
                        } else {
                            self.move_keyboard_focus(1);
                        }
                    }
                }
            }
            WindowEvent::Ime(ime) => {
                trace!("IME event: {ime:?}");
                if let Ime::Enabled = &ime {
                    let shared = self.shared_window_data.0.borrow();
                    shared.winit_window.set_ime_cursor_area(
                        PhysicalPosition::new(
                            shared.ime_cursor_area.top_left.x,
                            shared.ime_cursor_area.top_left.y,
                        ),
                        PhysicalSize::new(
                            shared.ime_cursor_area.size.x,
                            shared.ime_cursor_area.size.y,
                        ),
                    );
                }
                // TODO: deduplicate with ReceivedCharacter
                let focused_widget1 = self.shared_window_data.0.borrow().focused_widget.clone();
                if let Some(focused_widget) = focused_widget1 {
                    if let Ok(widget) = get_widget_by_id_mut(self.root_widget, focused_widget.1) {
                        widget.dispatch(ImeEvent::new(ime).into());
                    }
                }
                //self.inner.set_ime_position(PhysicalPosition::new(10, 10));
            }
            WindowEvent::Focused(focused) => {
                self.shared_window_data.0.borrow_mut().is_window_focused = focused;
                let mouse_grabber_widget_is_some1 = self
                    .shared_window_data
                    .0
                    .borrow()
                    .mouse_grabber_widget
                    .is_some();
                if !focused && mouse_grabber_widget_is_some1 {
                    self.shared_window_data.0.borrow_mut().mouse_grabber_widget = None;
                    self.dispatch_cursor_leave();
                }
                self.root_widget
                    .dispatch(WindowFocusChangeEvent::new(focused).into());
            }
            _ => {}
        }
        self.after_widget_activity();
    }

    pub fn handle_accesskit_event(&mut self, event: accesskit_winit::Event) {
        match event.window_event {
            accesskit_winit::WindowEvent::InitialTreeRequested => {
                self.push_accessible_updates();
            }
            accesskit_winit::WindowEvent::ActionRequested(request) => {
                trace!("accesskit request: {:?}", request);
                self.handle_accessible_request(request);
            }
            accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
        }
    }

    pub fn after_widget_activity(&mut self) {
        self.push_accessible_updates();
        let pending_size_hint_invalidations = mem::take(
            &mut self
                .shared_window_data
                .0
                .borrow_mut()
                .pending_size_hint_invalidations,
        );
        if !pending_size_hint_invalidations.is_empty() {
            invalidate_size_hint_cache(self.root_widget, &pending_size_hint_invalidations);
            self.layout(pending_size_hint_invalidations);
        }

        let focusable_widgets_changed1 =
            self.shared_window_data.0.borrow().focusable_widgets_changed;
        if focusable_widgets_changed1 {
            //println!("focusable_widgets_changed!");
            self.shared_window_data
                .0
                .borrow_mut()
                .focusable_widgets_changed = false;

            let focused_widget1: Option<(WidgetAddress, RawWidgetId)> =
                self.shared_window_data.0.borrow().focused_widget.clone();
            if let Some(focused_widget) = focused_widget1 {
                let is_err1 = self
                    .shared_window_data
                    .0
                    .borrow()
                    .focusable_widgets
                    .binary_search(&focused_widget)
                    .is_err();
                if is_err1 {
                    self.unset_focus();
                }
            }
            self.check_auto_focus();
        }
        // TODO: may need another turn of `after_widget_activity()`
    }

    pub fn move_keyboard_focus(&mut self, direction: i32) {
        let focused_widget1 = self.shared_window_data.0.borrow().focused_widget.clone();
        if let Some(focused_widget) = focused_widget1 {
            if let Some(new_addr_id) = self
                .shared_window_data
                .move_keyboard_focus(&focused_widget, direction)
            {
                self.set_focus(new_addr_id, FocusReason::Tab);
            } else {
                self.unset_focus();
            }
        }
        self.check_auto_focus();
    }

    // pub fn set_widget(&mut self, mut widget: Box<dyn Widget>) {
    //     self.root_widget.dispatch(UnmountEvent::new().into());

    //     let address = WidgetAddress::window_root(self.id);
    //     widget.dispatch(
    //         MountEvent::new(MountPoint {
    //             address,
    //             parent_id: None,
    //             window: self.shared_window_data.clone(),
    //             index_in_parent: 0,
    //         })
    //         .into(),
    //     );
    //     self.root_widget = widget;
    // }

    fn push_accessible_updates(&mut self) {
        let data = &mut *self.shared_window_data.0.borrow_mut();
        data.accesskit_adapter
            .update_if_active(|| data.accessible_nodes.take_update());
    }

    fn check_auto_focus(&mut self) {
        let is_none1 = self.shared_window_data.0.borrow().focused_widget.is_none();
        if is_none1 {
            let id = self
                .shared_window_data
                .0
                .borrow()
                .focusable_widgets
                .first()
                .cloned();
            if let Some(id) = id {
                self.set_focus(id, FocusReason::Auto);
            }
        }
    }

    fn set_focus(&mut self, widget_addr_id: (WidgetAddress, RawWidgetId), reason: FocusReason) {
        if let Ok(widget) = get_widget_by_id_mut(self.root_widget, widget_addr_id.1) {
            if !widget.common().is_focusable {
                warn!("cannot focus widget that is not focusable");
                return;
            }
            let allowed = widget.common().enable_ime;
            self.shared_window_data
                .0
                .borrow()
                .winit_window
                .set_ime_allowed(allowed);
            self.shared_window_data.0.borrow_mut().ime_allowed = allowed;
        } else {
            warn!("set_focus: widget not found");
        }

        let old_widget_id1 = self.shared_window_data.0.borrow_mut().focused_widget.take();
        if let Some(old_widget_id) = old_widget_id1 {
            self.shared_window_data
                .0
                .borrow_mut()
                .accessible_nodes
                .set_focus(None);
            if let Ok(old_widget) = get_widget_by_id_mut(self.root_widget, old_widget_id.1) {
                old_widget.dispatch(FocusOutEvent::new().into());
            }
        }

        if let Ok(widget) = get_widget_by_id_mut(self.root_widget, widget_addr_id.1) {
            widget.dispatch(FocusInEvent::new(reason).into());
            self.shared_window_data
                .0
                .borrow_mut()
                .accessible_nodes
                .set_focus(Some(widget_addr_id.1.into()));
            self.shared_window_data.0.borrow_mut().focused_widget = Some(widget_addr_id);
        } else {
            warn!("set_focus: widget not found on second pass");
        }
    }

    fn unset_focus(&mut self) {
        self.shared_window_data.0.borrow_mut().focused_widget = None;
        self.shared_window_data
            .0
            .borrow()
            .winit_window
            .set_ime_allowed(false);
        self.shared_window_data.0.borrow_mut().ime_allowed = false;
        self.shared_window_data
            .0
            .borrow_mut()
            .accessible_nodes
            .set_focus(None);
    }

    fn layout(&mut self, changed_size_hints: Vec<WidgetAddress>) {
        let inner_size = self.shared_window_data.0.borrow().winit_window.inner_size();
        self.root_widget.dispatch(
            LayoutEvent {
                new_rect_in_window: Some(Rect {
                    top_left: Point::default(),
                    size: Size {
                        x: inner_size.width as i32,
                        y: inner_size.height as i32,
                    },
                }),
                changed_size_hints,
            }
            .into(),
        );
    }

    pub fn handle_request(&mut self, request: WindowRequest) {
        match request {
            WindowRequest::SetFocus(request) => {
                let Some(addr) = address(request.widget_id) else {
                    warn!("cannot focus unmounted widget");
                    return;
                };
                let pair = (addr, request.widget_id);
                let is_err1 = self
                    .shared_window_data
                    .0
                    .borrow()
                    .focusable_widgets
                    .binary_search(&pair)
                    .is_err();
                if is_err1 {
                    warn!("cannot focus widget: not registered as focusable");
                    return;
                }
                self.set_focus(pair, request.reason);
            }
        }
        self.push_accessible_updates();
    }

    pub fn handle_accessible_request(&mut self, request: ActionRequest) {
        let root = self.shared_window_data.0.borrow().accessible_nodes.root();
        if request.target == root {
            warn!("cannot dispatch accessible event to virtual root: {request:?}");
            return;
        }
        let widget_id = RawWidgetId(request.target.0);
        if let Ok(widget) = get_widget_by_id_mut(self.root_widget, widget_id) {
            widget.dispatch(AccessibleActionEvent::new(request.action, request.data).into());
        } else {
            warn!("cannot dispatch accessible event (no such widget): {request:?}");
        }
    }
}

#[derive(Debug, From)]
pub enum WindowRequest {
    SetFocus(SetFocusRequest),
}

#[derive(Debug)]
pub struct SetFocusRequest {
    pub widget_id: RawWidgetId,
    pub reason: FocusReason,
}
