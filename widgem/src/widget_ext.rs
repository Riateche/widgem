use {
    crate::{
        callback::Callback,
        event::{Event, LayoutEvent, StyleChangeEvent},
        layout::{Layout, SizeHint, FALLBACK_SIZE_HINTS},
        style::css::PseudoClass,
        system::{LayoutState, OrWarn},
        types::PhysicalPixels,
        ScrollToRectRequest, Widget, WidgetGeometry, WidgetId,
    },
    anyhow::Result,
    std::borrow::Cow,
    tracing::warn,
};

pub trait WidgetExt: Widget {
    fn id(&self) -> WidgetId<Self>
    where
        Self: Sized,
    {
        WidgetId::new(self.base().id())
    }

    fn set_visible(&mut self, value: bool) -> &mut Self {
        self.base_mut().set_visible(value);
        self
    }

    fn set_focusable(&mut self, value: bool) -> &mut Self {
        self.base_mut().set_focusable(value);
        self
    }
    fn set_accessibility_node_enabled(&mut self, value: bool) -> &mut Self {
        self.base_mut().set_accessibility_node_enabled(value);
        self
    }

    fn set_style(&mut self, style: &str) -> &mut Self {
        self.base_mut().set_style(style);
        self
    }

    fn callback<F, E>(&self, func: F) -> Callback<E>
    where
        F: Fn(&mut Self, E) -> Result<()> + 'static,
        E: 'static,
        Self: Sized,
    {
        self.base().app().create_widget_callback(self.id(), func)
    }

    fn dispatch(&mut self, event: Event) -> bool {
        let mut accepted = false;
        let should_dispatch = if self.base().is_enabled() {
            true
        } else {
            match &event {
                Event::MouseInput(_)
                | Event::MouseScroll(_)
                | Event::MouseEnter(_)
                | Event::MouseMove(_)
                | Event::MouseLeave(_)
                | Event::KeyboardInput(_)
                | Event::InputMethod(_)
                | Event::Activate(_) => false,
                Event::Draw(_)
                | Event::Layout(_)
                | Event::FocusIn(_)
                | Event::FocusOut(_)
                | Event::WindowFocusChange(_)
                | Event::AccessibilityAction(_)
                | Event::StyleChange(_) => true,
            }
        };
        if should_dispatch {
            if self.base_mut().before_event(&event) {
                accepted = true;
            }
        }
        match &event {
            Event::MouseMove(event) => {
                if !accepted && should_dispatch {
                    let is_enter = if let Some(window) = self.base().window_or_err().or_warn() {
                        !window.is_mouse_entered(self.base().id())
                    } else {
                        false
                    };

                    if is_enter {
                        self.dispatch(event.create_enter_event().into());
                    }
                }
            }
            Event::MouseEnter(_) => {
                self.add_pseudo_class(PseudoClass::Hover);
            }
            Event::MouseLeave(_) => {
                self.remove_pseudo_class(PseudoClass::Hover);
            }
            Event::FocusIn(_) => {
                self.set_pseudo_class(PseudoClass::Focus, self.base().is_window_focused());
            }
            Event::FocusOut(_) => {
                self.remove_pseudo_class(PseudoClass::Focus);
            }
            Event::WindowFocusChange(event) => {
                self.set_pseudo_class(
                    PseudoClass::Focus,
                    event.is_window_focused && self.base().is_focused(),
                );
            }
            _ => (),
        }
        if !accepted && should_dispatch {
            accepted = self.handle_event(event.clone()).or_warn().unwrap_or(false);
        }
        match event {
            Event::MouseInput(_) | Event::MouseScroll(_) => {
                if accepted {
                    let common = self.base_mut();
                    if let Some(window) = common.window_or_err().or_warn() {
                        if window
                            .current_mouse_event_state()
                            .or_warn()
                            .is_some_and(|e| !e.is_accepted())
                        {
                            window.accept_current_mouse_event(common.id()).or_warn();
                        }
                    }
                }
            }
            Event::MouseEnter(_) => {
                // TODO: rename or rework to only accept if handler returned true
                if let Some(rect_in_window) = self.base().rect_in_window_or_err().or_warn() {
                    if let Some(window) = self.base().window_or_err().or_warn() {
                        window.add_mouse_entered(rect_in_window, self.base().id());
                    }
                }
            }
            Event::MouseMove(_) => {
                if let Some(window) = self.base_mut().window_or_err().or_warn().cloned() {
                    if window
                        .current_mouse_event_state()
                        .or_warn()
                        .is_some_and(|e| !e.is_accepted())
                    {
                        window
                            .accept_current_mouse_event(self.base().id())
                            .or_warn();
                        window.set_cursor(self.base().cursor_icon());
                    }
                }
            }
            Event::Draw(event) => {
                for child in self.base_mut().children_mut() {
                    if let Some(rect_in_parent) = child.base().rect_in_parent() {
                        if let Some(child_event) = event.map_to_child(rect_in_parent) {
                            child.dispatch(child_event.into());
                        }
                    }
                }
            }
            Event::WindowFocusChange(event) => {
                for child in self.base_mut().children_mut() {
                    child.dispatch(event.clone().into());
                }
            }
            Event::FocusIn(_) | Event::FocusOut(_) | Event::MouseLeave(_) => {
                self.base_mut().update();
            }
            Event::Layout(_) => {
                self.base_mut().update();
                let state = self
                    .base()
                    .app()
                    .with_current_layout_state(|state| state.clone())
                    .unwrap_or_else(|| {
                        warn!("WidgetExt::dispatch: missing layout_state");
                        LayoutState::default()
                    });
                for child in self.base_mut().children_mut() {
                    if state
                        .changed_size_hints
                        .iter()
                        .any(|changed| changed.starts_with(child.base().address()))
                    {
                        let geometry = child.base().geometry().cloned();
                        child.dispatch(
                            LayoutEvent {
                                new_geometry: geometry,
                            }
                            .into(),
                        );
                    }
                }
            }
            Event::StyleChange(event) => {
                for child in self.base_mut().children_mut() {
                    // TODO: only if really changed
                    child.dispatch(event.clone().into());
                }
            }
            Event::KeyboardInput(_)
            | Event::InputMethod(_)
            | Event::AccessibilityAction(_)
            | Event::Activate(_) => {}
        }

        self.update_accessibility_node();
        accepted
    }

    fn scroll_to_rect(&mut self, request: ScrollToRectRequest) -> bool {
        let accepted = self
            .handle_scroll_to_rect_request(request.clone())
            .or_warn()
            .unwrap_or(false);
        if accepted {
            // TODO: propagate to inner widgets anyway? how does it work with two scroll areas?
            return true;
        }
        if &request.address != self.base().address() {
            if request.address.starts_with(self.base().address()) {
                if let Some((key, id)) = request.address.item_at(self.base().address().len()) {
                    if let Ok(child) = self.base_mut().get_dyn_child_mut(key) {
                        if &child.base().id() == id {
                            child.scroll_to_rect(request.clone());
                        } else {
                            warn!("child id mismatch while dispatching ScrollToRectEvent");
                        }
                    } else {
                        warn!("invalid child index while dispatching ScrollToRectEvent");
                    }
                } else {
                    warn!("couldn't get child index while dispatching ScrollToRectEvent");
                }
            } else {
                warn!("ScrollToRectEvent dispatched to unrelated widget");
            }
        }

        false
    }

    fn update_accessibility_node(&mut self) {
        let node = if self.base().is_accessibility_node_enabled() {
            self.handle_accessibility_node_request().or_warn().flatten()
        } else {
            None
        };

        let Some(window) = self.base().window() else {
            return;
        };
        // TODO: refresh after layout event
        let rect = self.base().rect_in_window();
        let node = node.map(|mut node| {
            if let Some(rect) = rect {
                node.set_bounds(rect.into());
            }
            node
        });
        window.accessibility_node_updated(self.base().id().into(), node);
    }

    fn update_children(&mut self) {
        if !self.base().has_declare_children_override() {
            return;
        }
        self.handle_declare_children_request().or_warn();
    }

    fn size_hint_x(&self, size_y: Option<PhysicalPixels>) -> SizeHint {
        if let Some(cached) = self.base().size_hint_x_cache(size_y) {
            cached
        } else {
            let r = self
                .handle_size_hint_x_request(size_y)
                .or_warn()
                .unwrap_or(FALLBACK_SIZE_HINTS);
            self.base().set_size_hint_x_cache(size_y, r);
            r
        }
    }

    fn size_hint_y(&self, size_x: PhysicalPixels) -> SizeHint {
        if let Some(cached) = self.base().size_hint_y_cache(size_x) {
            cached
        } else {
            let r = self
                .handle_size_hint_y_request(size_x)
                .or_warn()
                .unwrap_or(FALLBACK_SIZE_HINTS);
            self.base().set_size_hint_y_cache(size_x, r);
            r
        }
    }

    fn add_class(&mut self, class: Cow<'static, str>) -> &mut Self {
        self.base_mut().add_class(class);
        self
    }

    fn set_padding_enabled(&mut self, padding_enabled: bool) -> &mut Self {
        self.set_class(Cow::Borrowed("no_padding"), !padding_enabled)
    }

    fn remove_class(&mut self, class: Cow<'static, str>) -> &mut Self {
        self.base_mut().remove_class(class);
        self
    }

    fn has_class(&self, class: &str) -> bool {
        self.base().style_selector().has_class(class)
    }

    fn set_class(&mut self, class: Cow<'static, str>, present: bool) -> &mut Self {
        self.base_mut().set_class(class, present);
        self
    }

    fn add_pseudo_class(&mut self, class: PseudoClass) -> &mut Self {
        self.base_mut().add_pseudo_class(class);
        self
    }

    fn remove_pseudo_class(&mut self, class: PseudoClass) -> &mut Self {
        self.base_mut().remove_pseudo_class(class);
        self
    }

    fn has_pseudo_class(&self, class: PseudoClass) -> bool {
        self.base().has_pseudo_class(class)
    }

    fn set_pseudo_class(&mut self, class: PseudoClass, present: bool) -> &mut Self {
        self.base_mut().set_pseudo_class(class, present);
        self
    }

    fn set_enabled(&mut self, enabled: bool) -> &mut Self {
        self.base_mut().set_enabled(enabled);
        self
    }

    fn set_scale(&mut self, scale: Option<f32>) -> &mut Self {
        if self.base().self_scale() == scale {
            return self;
        }
        self.base_mut().set_scale(scale);
        self.dispatch(StyleChangeEvent { _empty: () }.into());
        self
    }

    /// Assign column `x` and row `y` to this widget in the parent widget's grid.
    ///
    /// Same as [WidgetBase::set_grid_cell](crate::WidgetBase::set_grid_cell).
    fn set_grid_cell(&mut self, x: i32, y: i32) -> &mut Self {
        self.base_mut().set_grid_cell(x, y);
        self
    }

    fn set_size_x_fixed(&mut self, fixed: Option<bool>) -> &mut Self {
        let mut options = self.base().layout_item_options().clone();
        options.set_x_fixed(fixed);
        self.base_mut().set_layout_item_options(options);
        self
    }
    fn set_size_y_fixed(&mut self, fixed: Option<bool>) -> &mut Self {
        let mut options = self.base().layout_item_options().clone();
        options.set_y_fixed(fixed);
        self.base_mut().set_layout_item_options(options);
        self
    }

    fn set_geometry(&mut self, geometry: Option<WidgetGeometry>) -> &mut Self {
        if self.base().geometry() == geometry.as_ref() {
            return self;
        }
        self.base_mut().set_geometry(geometry.clone());
        let had_layout_state = self.base().app().with_current_layout_state(|state| {
            if let Some(layout_state) = state {
                layout_state
                    .changed_size_hints
                    .push(self.base().address().clone());
                return true;
            }
            *state = Some(LayoutState {
                changed_size_hints: Vec::new(),
            });
            false
        });
        if !had_layout_state {
            self.dispatch(
                LayoutEvent {
                    new_geometry: geometry,
                }
                .into(),
            );
            self.base().app().with_current_layout_state(|state| {
                if state.is_none() {
                    warn!("WidgetExt::set_geometry: layout state is missing in system data");
                }
                *state = None;
            });
        }
        self
    }

    fn set_layout(&mut self, layout: Layout) -> &mut Self {
        self.base_mut().set_layout(layout);
        self
    }

    fn boxed(self) -> Box<dyn Widget>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

impl<W: Widget + ?Sized> WidgetExt for W {}
