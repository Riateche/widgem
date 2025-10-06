use {
    crate::{
        event::{
            AccessibilityActionEvent, FocusInEvent, FocusOutEvent, FocusReason, InputMethodEvent,
            KeyboardInputEvent, LayoutEvent, MouseInputEvent, MouseLeaveEvent, MouseMoveEvent,
            MouseScrollEvent, StyleChangeEvent, WindowFocusChangeEvent,
        },
        shared_window::{MouseEventState, SharedWindow, WindowRequest},
        system::{LayoutState, OrWarn},
        types::{PhysicalPixels, Point, Size},
        widgets::{get_widget_by_address_mut, get_widget_by_id_mut, invalidate_size_hint_cache},
        RawWidgetId, ScrollToRectRequest, Widget, WidgetAddress, WidgetExt, WidgetGeometry,
    },
    accesskit::ActionRequest,
    std::cmp::max,
    tracing::{trace, warn},
    winit::{
        event::{ElementState, Ime, WindowEvent},
        keyboard::{Key, NamedKey},
        window::CursorIcon,
    },
};

pub struct WindowHandler<'a> {
    pub window: SharedWindow,
    pub root_widget: &'a mut dyn Widget,
}

impl<'a> WindowHandler<'a> {
    pub fn new(window: SharedWindow, root_widget: &'a mut dyn Widget) -> Self {
        WindowHandler {
            window,
            root_widget,
        }
    }

    pub(crate) fn dispatch_mouse_leave(&mut self) {
        //println!("\n------dispatch_mouse_leave");
        while let Some(id) = self.window.pop_mouse_entered_widget() {
            //println!("pop {id:?}");
            if let Ok(widget) = get_widget_by_id_mut(self.root_widget, id) {
                widget.dispatch(MouseLeaveEvent { _empty: () }.into());
            }
        }
    }

    pub fn handle_event(&mut self, event: WindowEvent) {
        self.window.pass_event_to_accesskit(&event);

        match event {
            WindowEvent::RedrawRequested => {
                if let Some(draw_event) = self.window.prepare_draw(self.root_widget.base().app()) {
                    self.root_widget.dispatch(draw_event.into());
                }
                self.window.finalize_draw();
            }
            WindowEvent::Resized(_) => {
                self.layout(Vec::new());
            }
            WindowEvent::CloseRequested => {
                self.window.close();
            }
            // TODO: should use device id?
            WindowEvent::CursorEntered { .. } => {
                self.window.cursor_entered();
            }
            WindowEvent::CursorLeft { .. } => {
                self.window.cursor_left();
                self.dispatch_mouse_leave();
            }
            WindowEvent::CursorMoved {
                position,
                device_id,
                ..
            } => {
                // At least on macos, we have to do `floor` to match the way the mouse pointer is displayed.
                let pos_in_window = Point::new(
                    PhysicalPixels::from_i32(position.x.floor() as i32),
                    PhysicalPixels::from_i32(position.y.floor() as i32),
                );
                if !self.window.cursor_moved(pos_in_window) {
                    return;
                }
                self.dispatch_mouse_leave();

                self.window.init_mouse_event_state().or_warn();
                if let Some(mouse_grabber_widget_id) = self.window.mouse_grabber_widget() {
                    if let Ok(mouse_grabber_widget) =
                        get_widget_by_id_mut(self.root_widget, mouse_grabber_widget_id)
                    {
                        if let Some(rect_in_window) = mouse_grabber_widget.base().rect_in_window() {
                            let pos_in_widget = pos_in_window - rect_in_window.top_left();
                            mouse_grabber_widget.dispatch(
                                MouseMoveEvent {
                                    device_id,
                                    pos: pos_in_widget,
                                    pos_in_window,
                                }
                                .into(),
                            );
                        }
                    }
                } else {
                    self.root_widget.dispatch(
                        MouseMoveEvent {
                            device_id,
                            pos: pos_in_window,
                            pos_in_window,
                        }
                        .into(),
                    );
                }
                let state = self.window.take_mouse_event_state().or_warn();
                if state.is_some_and(|state| !state.is_accepted()) {
                    self.window.set_cursor(CursorIcon::Default);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.window.set_modifiers(modifiers.state());
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
                ..
            } => {
                self.window.mouse_input(state, button);
                if let Some(pos_in_window) = self.window.cursor_position() {
                    self.window.init_mouse_event_state().or_warn();
                    if let Some(mouse_grabber_widget_id) = self.window.mouse_grabber_widget() {
                        if let Ok(mouse_grabber_widget) =
                            get_widget_by_id_mut(self.root_widget, mouse_grabber_widget_id)
                        {
                            if let Some(rect_in_window) =
                                mouse_grabber_widget.base().rect_in_window()
                            {
                                let pos_in_widget = pos_in_window - rect_in_window.top_left();
                                let event = MouseInputEvent {
                                    device_id,
                                    state,
                                    button,
                                    num_clicks: self.window.num_clicks(),
                                    pos: pos_in_widget,
                                    pos_in_window,
                                };
                                mouse_grabber_widget.dispatch(event.into());
                            }
                        }
                        if !self.window.any_mouse_buttons_pressed() {
                            self.window.set_mouse_grabber_widget(None);
                            self.dispatch_mouse_leave();
                        }
                    } else {
                        let event = MouseInputEvent {
                            device_id,
                            state,
                            button,
                            num_clicks: self.window.num_clicks(),
                            pos: pos_in_window,
                            pos_in_window,
                        };
                        self.root_widget.dispatch(event.into());
                    }
                    {
                        if let Some(event_state) = self.window.take_mouse_event_state().or_warn() {
                            if state == ElementState::Pressed
                                && self.window.mouse_grabber_widget().is_none()
                            {
                                if let MouseEventState::AcceptedBy(accepted_by_widget_id) =
                                    event_state
                                {
                                    self.window
                                        .set_mouse_grabber_widget(Some(accepted_by_widget_id));
                                }
                            }
                        }
                    }
                } else {
                    warn!("no cursor position in mouse input handler");
                }
            }
            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
            } => {
                if let Some(pos_in_window) = self.window.cursor_position() {
                    self.window.init_mouse_event_state().or_warn();
                    if let Some(mouse_grabber_widget_id) = self.window.mouse_grabber_widget() {
                        if let Ok(mouse_grabber_widget) =
                            get_widget_by_id_mut(self.root_widget, mouse_grabber_widget_id)
                        {
                            if let Some(rect_in_window) =
                                mouse_grabber_widget.base().rect_in_window()
                            {
                                let pos_in_widget = pos_in_window - rect_in_window.top_left();
                                let event = MouseScrollEvent {
                                    device_id,
                                    delta,
                                    touch_phase: phase,
                                    pos: pos_in_widget,
                                    pos_in_window,
                                };
                                mouse_grabber_widget.dispatch(event.into());
                            }
                        }
                        if !self.window.any_mouse_buttons_pressed() {
                            self.window.set_mouse_grabber_widget(None);
                            self.dispatch_mouse_leave();
                        }
                    } else {
                        let event = MouseScrollEvent {
                            device_id,
                            delta,
                            touch_phase: phase,
                            pos: pos_in_window,
                            pos_in_window,
                        };
                        self.root_widget.dispatch(event.into());
                    }
                    self.window.take_mouse_event_state().or_warn();
                    // TODO: should we dispatch to focused widget on Windows by default?
                    // Qt dispatches the event to focused widget if moused-over widget did not accept it.
                } else {
                    warn!("no cursor position in mouse wheel handler");
                }
            }
            WindowEvent::KeyboardInput {
                device_id,
                is_synthetic,
                event,
            } => {
                let event = KeyboardInputEvent {
                    device_id,
                    info: event.clone(),
                    is_synthetic,
                    modifiers: self.window.modifiers(),
                };
                if let Some(focused_widget) = self.window.focused_widget() {
                    if let Ok(widget) = get_widget_by_id_mut(self.root_widget, focused_widget) {
                        widget.dispatch(event.clone().into());
                    }
                }

                // TODO: only if event is not accepted by a widget
                if event.info.state == ElementState::Pressed {
                    let logical_key = &event.info.logical_key;
                    if logical_key == &Key::Named(NamedKey::Tab) {
                        if self.window.modifiers().shift_key() {
                            self.move_keyboard_focus(-1);
                        } else {
                            self.move_keyboard_focus(1);
                        }
                    }
                }

                // TODO: only if event is not accepted above
                self.root_widget.base().app().trigger_shortcuts(&event);
            }
            WindowEvent::Ime(ime) => {
                trace!("IME event: {ime:?}");
                if let Ime::Enabled = &ime {
                    self.window.ime_enabled();
                }
                // TODO: deduplicate with ReceivedCharacter
                if let Some(focused_widget) = self.window.focused_widget() {
                    if let Ok(widget) = get_widget_by_id_mut(self.root_widget, focused_widget) {
                        widget.dispatch(InputMethodEvent { info: ime }.into());
                    }
                }
                //self.inner.set_ime_position(PhysicalPosition::new(10, 10));
            }
            WindowEvent::Focused(is_focused) => {
                trace!("window focus {:?} {}", self.window.id(), is_focused);
                if self.window.focus_changed(is_focused) {
                    self.dispatch_mouse_leave();
                }
                self.root_widget.dispatch(
                    WindowFocusChangeEvent {
                        is_window_focused: is_focused,
                    }
                    .into(),
                );
            }
            _ => {}
        }
        self.after_widget_activity();
    }

    pub fn handle_accesskit_event(&mut self, event: accesskit_winit::Event) {
        match event.window_event {
            accesskit_winit::WindowEvent::InitialTreeRequested => {
                self.window.push_accessibility_updates();
            }
            accesskit_winit::WindowEvent::ActionRequested(request) => {
                trace!("accesskit request: {:?}", request);
                self.handle_accessibility_request(request);
            }
            accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
        }
    }

    pub fn after_widget_activity(&mut self) {
        let mut should_layout = false;
        let style_changes = self.window.take_pending_style_change_events();
        for addr in style_changes {
            let Some(widget) = get_widget_by_address_mut(self.root_widget, &addr).or_warn() else {
                continue;
            };
            widget.dispatch(StyleChangeEvent { _empty: () }.into());
        }
        if !self.window.has_winit_window() {
            self.window.init_winit_window(self.root_widget);
            should_layout = true;
        }
        let accessibility_updates = self.window.take_pending_accessibility_updates();
        for addr in accessibility_updates {
            let Some(widget) = get_widget_by_address_mut(self.root_widget, &addr).or_warn() else {
                continue;
            };
            widget.update_accessibility_node();
        }
        self.window.push_accessibility_updates();
        let pending_size_hint_invalidations = self.window.take_pending_size_hint_invalidations();
        if !pending_size_hint_invalidations.is_empty() {
            invalidate_size_hint_cache(self.root_widget, &pending_size_hint_invalidations);
            should_layout = true;
        }
        if should_layout {
            self.layout(pending_size_hint_invalidations);
        }
        if self.window.focusable_widgets_changed() {
            self.window.clear_focusable_widgets_changed();
            if !self.window.focused_widget_is_focusable() {
                self.unset_focus();
            }
            self.check_auto_focus();
        }
        // TODO: may need another turn of `after_widget_activity()`
    }

    fn unset_focus(&mut self) {
        if let Some(old_widget_id) = self.window.unset_focus() {
            if let Ok(old_widget) = get_widget_by_id_mut(self.root_widget, old_widget_id.1) {
                old_widget.dispatch(FocusOutEvent { _empty: () }.into());
            }
        }
    }

    pub fn move_keyboard_focus(&mut self, direction: i32) {
        if let Some(new_addr_id) = self.window.move_keyboard_focus(direction) {
            self.set_focus(new_addr_id, FocusReason::Tab);
        } else {
            self.unset_focus();
        }
        self.check_auto_focus();
    }

    fn check_auto_focus(&mut self) {
        if let Some(id) = self.window.pending_auto_focus() {
            self.set_focus(id, FocusReason::Auto);
        }
    }

    fn set_focus(
        &mut self,
        widget_addr_id: (Vec<(crate::child_key::ChildKey, RawWidgetId)>, RawWidgetId),
        reason: FocusReason,
    ) {
        if let Ok(widget) = get_widget_by_id_mut(self.root_widget, widget_addr_id.1) {
            if !widget.base().is_focusable() {
                warn!("cannot focus widget that is not focusable");
                return;
            }
        } else {
            warn!("set_focus: widget not found");
        }

        if let Some(old_widget_id) = self.window.unset_focus() {
            if let Ok(old_widget) = get_widget_by_id_mut(self.root_widget, old_widget_id.1) {
                old_widget.dispatch(FocusOutEvent { _empty: () }.into());
            }
        }

        if let Ok(widget) = get_widget_by_id_mut(self.root_widget, widget_addr_id.1) {
            widget.dispatch(FocusInEvent { reason }.into());
            self.window
                .set_focus(widget_addr_id, widget.base().is_input_method_enabled());
        } else {
            warn!("set_focus: widget not found on second pass");
        }
    }

    fn layout(&mut self, changed_size_hints: Vec<WidgetAddress>) {
        let mut inner_size = self.window.inner_size();
        let old_min_size = self.window.min_inner_size();
        let old_preferred_size = self.window.preferred_inner_size();
        let hints_x = self.root_widget.size_hint_x(None);
        let preferred_size = Size::new(
            hints_x.preferred(),
            self.root_widget
                .size_hint_y(hints_x.preferred())
                .preferred(),
        );
        let min_size = Size::new(
            hints_x.min(),
            self.root_widget.size_hint_y(hints_x.min()).min(),
        );
        self.window.set_min_inner_size(min_size);
        trace!("window layout start, inner_size={:?}", inner_size);
        let mut resize_requested = false;
        if min_size != old_min_size || preferred_size != old_preferred_size {
            // Size hint of the widget changed.
            self.window.set_preferred_inner_size(preferred_size);
            if inner_size.x() < preferred_size.x() || inner_size.y() < preferred_size.y() {
                let new_size = Size::new(
                    max(inner_size.x(), preferred_size.x()),
                    max(inner_size.y(), preferred_size.y()),
                );
                trace!(
                    "requesting resize based on preferred size: \
                    inner_size={:?}, preferred_size={:?}, new_size={:?}",
                    inner_size,
                    preferred_size,
                    new_size
                );
                if let Some(response) = self.window.request_inner_size(new_size) {
                    inner_size = response;
                    trace!("resized to {:?}", inner_size);
                } else {
                    trace!("resize unsuccessful or deferred");
                }
                resize_requested = true;
            }
        }
        // Window somehow ended up smaller than the min size.
        if !resize_requested && (inner_size.x() < min_size.x() || inner_size.y() < min_size.y()) {
            let new_size = Size::new(
                max(inner_size.x(), min_size.x()),
                max(inner_size.y(), min_size.y()),
            );
            trace!(
                "requesting resize based on min size: \
                inner_size={:?}, min_size={:?}, new_size={:?}",
                inner_size,
                preferred_size,
                new_size
            );
            if let Some(response) = self.window.request_inner_size(new_size) {
                inner_size = response;
                trace!("resized to {:?}", inner_size);
            } else {
                trace!("resize unsuccessful or deferred");
            }
        }

        self.root_widget
            .base()
            .app()
            .with_current_layout_state(|state| {
                if state.is_some() {
                    warn!("WindowHandler::layout: layout is already in progress");
                }
                *state = Some(LayoutState { changed_size_hints });
            });
        // TODO: set geometry to `None` when window is hidden.
        let new_geometry = Some(WidgetGeometry::root(inner_size));
        self.root_widget.set_geometry(new_geometry.clone());
        self.root_widget
            .dispatch(LayoutEvent { new_geometry }.into());
        self.root_widget
            .base()
            .app()
            .with_current_layout_state(|state| {
                if state.is_none() {
                    warn!("WindowHandler::layout: layout state is missing in system data");
                }
                *state = None;
            });
    }

    pub fn handle_request(&mut self, request: WindowRequest) {
        match request {
            WindowRequest::SetFocus(request) => {
                let Some(addr) = self.root_widget.base().app().address(request.widget_id) else {
                    warn!("cannot focus unmounted widget");
                    return;
                };
                let Some(relative_addr) = addr.strip_prefix(self.window.root_widget_id()) else {
                    warn!("SetFocus: address outside root");
                    return;
                };
                let pair = (relative_addr.to_vec(), request.widget_id);
                if !self.window.is_registered_as_focusable(&pair) {
                    warn!("cannot focus widget: not registered as focusable");
                    return;
                }
                self.set_focus(pair, request.reason);
            }
            WindowRequest::ScrollToRect(request) => {
                if let Some(address) = self.root_widget.base().app().address(request.widget_id) {
                    self.root_widget.scroll_to_rect(ScrollToRectRequest {
                        address,
                        rect: request.rect,
                    });
                } else {
                    warn!("ScrollToRectRequest: couldn't find widget address");
                }
            }
        }
        self.window.push_accessibility_updates();
    }

    pub fn handle_accessibility_request(&mut self, request: ActionRequest) {
        if request.target == self.window.root_accessibility_node_id() {
            warn!("cannot dispatch accessibility request to virtual root: {request:?}");
            return;
        }
        if let Ok(widget) = get_widget_by_id_mut(self.root_widget, request.target.into()) {
            widget.dispatch(
                AccessibilityActionEvent {
                    action: request.action,
                    data: request.data,
                }
                .into(),
            );
        } else {
            warn!("cannot dispatch accessibility request (no such widget): {request:?}");
        }
    }
}
