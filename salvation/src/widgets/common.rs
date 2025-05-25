use {
    super::{address, RawWidgetId, Widget, WidgetAddress, WidgetId, WidgetNotFound},
    crate::{
        callback::{widget_callback, Callback},
        event::Event,
        key::Key,
        layout::{
            grid::{GridAxisOptions, GridOptions},
            Alignment, LayoutItemOptions, SizeHints,
        },
        shortcut::{Shortcut, ShortcutId, ShortcutScope},
        style::{
            computed::{CommonComputedStyle, ComputedElementStyle, ComputedStyle},
            css::{Element, MyPseudoClass},
        },
        system::{
            register_address, request_children_update, unregister_address, with_system,
            ChildrenUpdateState,
        },
        types::{Point, Rect, Size},
        window::{Window, WindowId},
    },
    anyhow::{Context, Result},
    derivative::Derivative,
    log::{error, warn},
    std::{
        collections::{BTreeMap, HashMap, HashSet},
        fmt::Debug,
        marker::PhantomData,
        mem,
        ops::{Deref, DerefMut},
        rc::Rc,
    },
    winit::window::CursorIcon,
};

#[derive(Debug, Clone)]
pub struct WidgetCreationContext {
    pub parent_id: Option<RawWidgetId>,
    pub address: WidgetAddress,
    pub window: Option<Window>,
    pub parent_style: ComputedStyle,
    pub is_parent_enabled: bool,
    pub is_window_root: bool,
}

pub type EventFilterFn = dyn Fn(Event) -> Result<bool>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetGeometry {
    // Rect of this widget in parent coordinates.
    pub rect_in_parent: Rect,
    // Top left of the parent widget in window coordinates.
    pub parent_top_left_in_window: Point,
    // In this widget's coordinates.
    pub parent_visible_rect: Rect,
}

impl WidgetGeometry {
    pub fn new(parent: &WidgetGeometry, rect_in_parent: Rect) -> Self {
        Self {
            rect_in_parent,
            parent_top_left_in_window: parent.rect_in_parent.top_left
                + parent.parent_top_left_in_window,
            parent_visible_rect: parent.visible_rect(),
        }
    }

    pub fn size(&self) -> Size {
        self.rect_in_parent.size
    }

    pub fn rect_in_self(&self) -> Rect {
        Rect::from_pos_size(Point::default(), self.rect_in_parent.size)
    }

    pub fn rect_in_window(&self) -> Rect {
        self.rect_in_parent
            .translate(self.parent_top_left_in_window)
    }

    pub fn visible_rect(&self) -> Rect {
        self.parent_visible_rect
            .translate(-self.rect_in_parent.top_left)
            .intersect(self.rect_in_self())
    }
}

// TODO: use bitflags?
#[derive(Derivative)]
#[derivative(Debug)]
pub struct WidgetCommon {
    pub id: RawWidgetId,
    pub is_focusable: bool,
    pub enable_ime: bool,
    pub cursor_icon: CursorIcon,
    // If true, all mouse events from the parent propagate to this widget,
    // regardless of its boundaries.
    pub receives_all_mouse_events: bool,

    pub is_focused: bool,
    pub is_window_focused: bool,

    pub parent_id: Option<RawWidgetId>,
    pub address: WidgetAddress,
    pub window: Option<Window>,

    pub parent_style: ComputedStyle,
    pub is_parent_enabled: bool,
    pub is_self_enabled: bool,
    pub is_self_visible: bool,

    pub is_window_root: bool,

    pub is_mouse_over: bool,

    // Present if the widget is not hidden, and only after layout.
    pub geometry: Option<WidgetGeometry>,

    #[derivative(Debug = "ignore")]
    pub children: BTreeMap<Key, Box<dyn Widget>>,
    pub layout_item_options: LayoutItemOptions,

    pub size_hint_x_cache: Option<SizeHints>,
    // TODO: limit count
    pub size_hint_y_cache: HashMap<i32, SizeHints>,
    pub is_accessible: bool,

    pub self_style: Option<ComputedStyle>,

    pub is_registered_as_focusable: bool,
    // TODO: multiple filters?
    // TODO: accept/reject event from filter; option to run filter after on_event
    #[derivative(Debug = "ignore")]
    pub event_filter: Option<Box<EventFilterFn>>,
    pub grid_options: Option<GridOptions>,
    pub no_padding: bool,

    pub shortcuts: Vec<Shortcut>,
    pub style_element: Element,
    pub common_style: Rc<CommonComputedStyle>,

    pub num_added_children: u32,
    pub has_declare_children_override: bool,
    // Direct and indirect children created by last call of this widget's
    // `handle_update_children`.
    pub declared_children: HashSet<RawWidgetId>,

    // TODO: add setter
    pub send_signals_on_setter_calls: bool,
}

impl Drop for WidgetCommon {
    fn drop(&mut self) {
        unregister_address(self.id);
        // Drop and unmount children before unmounting self.
        self.children.clear();
        self.unmount_accessible();
        for shortcut in &self.shortcuts {
            // TODO: deregister widget/window shortcuts
            if shortcut.scope == ShortcutScope::Application {
                with_system(|system| system.application_shortcuts.retain(|s| s.id != shortcut.id));
            }
        }
    }
}

impl WidgetCommon {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<T: Widget>(ctx: WidgetCreationContext) -> WidgetCommonTyped<T> {
        let id = ctx.address.widget_id();
        register_address(id, ctx.address.clone());

        let style_element = Element::new(T::type_name());
        let common_style = ctx.parent_style.get_common(&style_element);
        let mut common = Self {
            id,
            parent_id: ctx.parent_id,
            address: ctx.address,
            is_window_focused: ctx.window.as_ref().map_or(false, |w| w.is_focused()),
            window: ctx.window,
            receives_all_mouse_events: false,
            parent_style: ctx.parent_style,
            self_style: None,
            is_focusable: false,
            is_focused: false,
            is_parent_enabled: ctx.is_parent_enabled,
            is_self_enabled: true,
            is_self_visible: true,
            is_mouse_over: false,
            enable_ime: false,
            geometry: None,
            cursor_icon: CursorIcon::Default,
            children: BTreeMap::new(),
            layout_item_options: LayoutItemOptions::default(),
            size_hint_x_cache: None,
            size_hint_y_cache: HashMap::new(),
            is_accessible: true,
            is_registered_as_focusable: false,
            event_filter: None,
            is_window_root: ctx.is_window_root,
            grid_options: None,
            no_padding: false,
            shortcuts: Vec::new(),
            style_element,
            common_style,
            has_declare_children_override: true,
            send_signals_on_setter_calls: false,
            num_added_children: 0,
            declared_children: Default::default(),
        };

        if let Some(window) = &common.window {
            let root_widget_id = window.root_widget_id();
            window.accessible_mount(
                if common.id == root_widget_id {
                    None
                } else if let Some(parent_id) = common.parent_id {
                    Some(parent_id.into())
                } else {
                    warn!("widget is not a window root so it must have a parent");
                    None
                },
                common.id.into(),
                // TODO: calculate visual index instead
                common
                    .address
                    .path
                    .last()
                    .map(|(key, _id)| key.clone())
                    .unwrap_or_else(|| {
                        warn!("WidgetCommon::new: empty address encountered");
                        "".into()
                    }),
            );
        }
        common.update();
        common.enabled_changed();
        common.focused_changed();
        common.mouse_over_changed();
        common.register_focusable();
        common.refresh_common_style();

        WidgetCommonTyped {
            common,
            _marker: PhantomData,
        }
    }

    pub fn window_id(&self) -> Option<WindowId> {
        self.window.as_ref().map(|w| w.id())
    }

    pub fn set_grid_options(&mut self, options: Option<GridOptions>) {
        self.grid_options = options;
        self.size_hint_changed();
    }

    pub fn set_no_padding(&mut self, value: bool) {
        self.no_padding = value;
        self.size_hint_changed();
    }

    pub fn grid_options(&self) -> GridOptions {
        self.grid_options.clone().unwrap_or_else(|| {
            let style = self.style();
            GridOptions {
                x: GridAxisOptions {
                    min_padding: if self.no_padding {
                        0
                    } else {
                        style.0.grid.min_padding.x
                    },
                    min_spacing: style.0.grid.min_spacing.x,
                    preferred_padding: if self.no_padding {
                        0
                    } else {
                        style.0.grid.preferred_padding.x
                    },
                    preferred_spacing: style.0.grid.preferred_spacing.x,
                    border_collapse: 0,
                    alignment: Alignment::Start,
                },
                y: GridAxisOptions {
                    min_padding: if self.no_padding {
                        0
                    } else {
                        style.0.grid.min_padding.y
                    },
                    min_spacing: style.0.grid.min_spacing.y,
                    preferred_padding: if self.no_padding {
                        0
                    } else {
                        style.0.grid.preferred_padding.y
                    },
                    preferred_spacing: style.0.grid.preferred_spacing.y,
                    border_collapse: 0,
                    alignment: Alignment::Start,
                },
            }
        })
    }

    pub fn is_self_visible(&self) -> bool {
        self.is_self_visible
    }

    pub fn is_self_enabled(&self) -> bool {
        self.is_self_enabled
    }

    pub fn is_enabled(&self) -> bool {
        self.is_parent_enabled && self.is_self_enabled
    }

    pub fn is_focusable(&self) -> bool {
        self.is_focusable && self.is_enabled()
    }

    pub fn is_focused(&self) -> bool {
        self.is_focused && self.is_window_focused && self.is_enabled()
    }

    pub fn style(&self) -> &ComputedStyle {
        self.self_style.as_ref().unwrap_or(&self.parent_style)
    }

    pub fn new_creation_context(
        &self,
        new_id: RawWidgetId,
        key: Key,
        root_of_window: Option<Window>,
    ) -> WidgetCreationContext {
        WidgetCreationContext {
            parent_id: Some(self.id),
            address: self.address.clone().join(key, new_id),
            is_window_root: root_of_window.is_some(),
            window: root_of_window.or_else(|| self.window.clone()),
            parent_style: self.style().clone(),
            is_parent_enabled: self.is_enabled(),
        }
    }

    // Request redraw and accessible update
    pub fn update(&mut self) {
        if let Some(window) = &self.window {
            window.request_redraw();
            window.request_accessible_update(self.address.clone());
        };
        request_children_update(self.address.clone());
    }

    pub fn has_child(&self, key: impl Into<Key>) -> bool {
        self.children.contains_key(&key.into())
    }

    pub fn add_child<T: Widget>(&mut self) -> &mut T {
        let key = self.num_added_children;
        self.num_added_children += 1;
        self.add_child_common(key.into(), false, None)
    }

    pub fn add_child_with_key<T: Widget>(&mut self, key: impl Into<Key>) -> &mut T {
        self.add_child_common(key.into(), false, None)
    }

    pub fn declare_child<T: Widget>(&mut self) -> &mut T {
        let key = with_system(|system| {
            let Some(state) = system.current_children_update.as_mut() else {
                warn!("declare_child called outside of handle_update_children");
                return 0;
            };
            let num = state.num_declared_children.entry(self.id).or_default();
            let key = *num;
            *num += 1;
            key
        });
        self.add_child_common(key.into(), true, None)
    }

    pub fn declare_child_with_key<T: Widget>(&mut self, key: impl Into<Key>) -> &mut T {
        self.add_child_common(key.into(), true, None)
    }

    fn add_child_common<T: Widget>(
        &mut self,
        key: Key,
        declare: bool,
        new_id: Option<RawWidgetId>,
    ) -> &mut T {
        if declare {
            if let Some(old_widget) = self
                .children
                .get_mut(&key)
                .and_then(|c| c.downcast_mut::<T>())
            {
                with_system(|system| {
                    if let Some(state) = &mut system.current_children_update {
                        state.declared_children.insert(old_widget.common().id);
                    } else {
                        warn!("declare_child shouldn't be used outside of declare_children()");
                    }
                });
                // Should be `return old_widget` but borrow checker is not smart enough.
                // Should we use `polonius-the-crab` crate?
                return self
                    .children
                    .get_mut(&key)
                    .and_then(|c| c.downcast_mut::<T>())
                    .unwrap();
            }
        }

        let new_id = new_id.unwrap_or_else(RawWidgetId::new);
        let ctx = if T::is_window_root_type() {
            let new_window = Window::new(new_id);
            self.new_creation_context(new_id, key.clone(), Some(new_window.clone()))
        } else {
            self.new_creation_context(new_id, key.clone(), None)
        };
        // This may delete the old widget.
        self.children
            .insert(key.clone(), Box::new(T::new(WidgetCommon::new::<T>(ctx))));
        self.size_hint_changed();
        let widget = self.children.get_mut(&key).unwrap();
        if declare {
            with_system(|system| {
                if let Some(state) = &mut system.current_children_update {
                    state.declared_children.insert(widget.common().id);
                } else {
                    warn!("declare_child shouldn't be used outside of declare_children()");
                }
            });
        }
        widget.downcast_mut().unwrap()
    }

    pub fn get_dyn_child(&self, key: impl Into<Key>) -> anyhow::Result<&dyn Widget> {
        Ok(self
            .children
            .get(&key.into())
            .context("no such key")?
            .as_ref())
    }

    pub fn get_dyn_child_mut(&mut self, key: impl Into<Key>) -> anyhow::Result<&mut dyn Widget> {
        Ok(self
            .children
            .get_mut(&key.into())
            .context("no such key")?
            .as_mut())
    }

    pub fn get_child<T: Widget>(&self, key: impl Into<Key>) -> anyhow::Result<&T> {
        self.children
            .get(&key.into())
            .context("no such key")?
            .downcast_ref()
            .context("child type mismatch")
    }

    pub fn get_child_mut<T: Widget>(&mut self, key: impl Into<Key>) -> anyhow::Result<&mut T> {
        self.children
            .get_mut(&key.into())
            .context("no such key")?
            .downcast_mut()
            .context("child type mismatch")
    }

    pub fn layout_item_options(&self) -> &LayoutItemOptions {
        &self.layout_item_options
    }

    pub fn set_layout_item_options(&mut self, options: LayoutItemOptions) {
        self.layout_item_options = options;
        self.size_hint_changed();
    }

    pub fn remove_child(&mut self, key: impl Into<Key>) -> Result<(), WidgetNotFound> {
        self.children.remove(&key.into()).ok_or(WidgetNotFound)?;
        self.size_hint_changed();
        Ok(())
    }

    pub fn remove_child_by_id(&mut self, id: RawWidgetId) -> Result<(), WidgetNotFound> {
        if id == self.id {
            warn!("remove_child_by_id: cannot delete self");
            return Err(WidgetNotFound);
        }
        let Some(address) = address(id) else {
            // Widget probably already deleted.
            return Err(WidgetNotFound);
        };
        if !address.starts_with(&self.address) {
            warn!("remove_child_by_id: address is not within widget");
            return Err(WidgetNotFound);
        }
        let Some(parent_id) = address.parent_widget_id() else {
            error!("remove_child_by_id: encountered root widget");
            return Err(WidgetNotFound);
        };
        if parent_id == self.id {
            return self.remove_child(&address.path.last().unwrap().0);
        }
        let remaining_path = &address.path[self.address.len()..];
        let Some(mut current_widget) = self
            .children
            .get_mut(&remaining_path[0].0)
            .map(|c| c.as_mut())
        else {
            return Err(WidgetNotFound);
        };
        for (key, _id) in &address.path[self.address.len() + 1..] {
            if current_widget.common().id == parent_id {
                return current_widget
                    .common_mut()
                    .remove_child(&address.path.last().unwrap().0);
            }
            current_widget = current_widget
                .common_mut()
                .children
                .get_mut(key)
                .ok_or(WidgetNotFound)?
                .as_mut();
        }
        error!("remove_child_by_id: did not reach parent widget");
        Err(WidgetNotFound)
    }

    pub fn size_hint_changed(&mut self) {
        self.clear_size_hint_cache();
        let Some(window) = &self.window else {
            return;
        };
        window.invalidate_size_hint(self.address.clone());
    }

    pub fn clear_size_hint_cache(&mut self) {
        self.size_hint_x_cache = None;
        self.size_hint_y_cache.clear();
    }

    pub fn window_or_err(&self) -> Result<&Window> {
        self.window.as_ref().context("no window")
    }

    pub fn address(&self) -> &WidgetAddress {
        &self.address
    }

    pub fn size(&self) -> Option<Size> {
        self.geometry.as_ref().map(|g| g.size())
    }

    pub fn rect_in_window(&self) -> Option<Rect> {
        self.geometry.as_ref().map(|g| g.rect_in_window())
    }

    pub fn rect_in_parent(&self) -> Option<Rect> {
        self.geometry.as_ref().map(|g| g.rect_in_parent)
    }

    pub fn visible_rect(&self) -> Option<Rect> {
        self.geometry.as_ref().map(|g| g.visible_rect())
    }

    pub fn geometry_or_err(&self) -> Result<&WidgetGeometry> {
        self.geometry.as_ref().context("no geometry")
    }

    pub fn rect_in_window_or_err(&self) -> Result<Rect> {
        Ok(self.geometry_or_err()?.rect_in_window())
    }

    pub fn size_or_err(&self) -> Result<Size> {
        Ok(self.geometry_or_err()?.size())
    }

    pub fn rect_in_self_or_err(&self) -> Result<Rect> {
        Ok(self.geometry_or_err()?.rect_in_self())
    }

    fn register_focusable(&mut self) {
        let is_focusable = self.is_focusable();
        if is_focusable != self.is_registered_as_focusable {
            if let Some(window) = &self.window {
                if is_focusable {
                    window.add_focusable_widget(self.address.clone(), self.id);
                } else {
                    window.remove_focusable_widget(self.address.clone(), self.id);
                }
                self.is_registered_as_focusable = is_focusable;
            } else {
                self.is_registered_as_focusable = false;
            }
        }
    }

    pub fn enabled_changed(&mut self) {
        self.register_focusable();
        self.focused_changed();
        self.mouse_over_changed();
        if self.is_enabled() {
            self.style_element
                .remove_pseudo_class(MyPseudoClass::Disabled);
            self.style_element.add_pseudo_class(MyPseudoClass::Enabled);
        } else {
            self.style_element
                .remove_pseudo_class(MyPseudoClass::Enabled);
            self.style_element.add_pseudo_class(MyPseudoClass::Disabled);
        }
        self.refresh_common_style();
    }

    pub fn focused_changed(&mut self) {
        if self.is_focused() {
            self.style_element.add_pseudo_class(MyPseudoClass::Focus);
        } else {
            self.style_element.remove_pseudo_class(MyPseudoClass::Focus);
        }
        self.refresh_common_style();
    }

    pub fn mouse_over_changed(&mut self) {
        if self.is_mouse_over {
            self.style_element.add_pseudo_class(MyPseudoClass::Hover);
        } else {
            self.style_element.remove_pseudo_class(MyPseudoClass::Hover);
        }
        self.refresh_common_style();
    }

    pub fn set_focusable(&mut self, focusable: bool) {
        self.is_focusable = focusable;
        self.register_focusable();
    }

    fn unmount_accessible(&mut self) {
        // println!("unmount_accessible {:?}", self.id);
        // for child in self.children.values_mut() {
        //     child.common_mut().unmount_accessible();
        // }
        if let Some(window) = &self.window {
            let root_widget_id = window.root_widget_id();
            window.accessible_unmount(
                if self.id == root_widget_id {
                    None
                } else {
                    self.parent_id.map(|id| id.into())
                },
                self.id.into(),
            );
        }
    }

    pub fn widget<W: Widget>(&mut self, id: WidgetId<W>) -> Result<&mut W, WidgetNotFound> {
        let w = self.widget_raw(id.0)?;
        Ok(w.downcast_mut::<W>().expect("widget downcast failed"))
    }

    pub fn widget_raw(&mut self, id: RawWidgetId) -> Result<&mut dyn Widget, WidgetNotFound> {
        // TODO: speed up
        for child in self.children.values_mut() {
            if child.common().id == id {
                return Ok(child.as_mut());
            }
            if let Ok(widget) = child.common_mut().widget_raw(id) {
                return Ok(widget);
            }
        }
        Err(WidgetNotFound)
    }

    pub fn add_shortcut(&mut self, shortcut: Shortcut) -> ShortcutId {
        let id = shortcut.id;
        if shortcut.scope == ShortcutScope::Application {
            with_system(|system| system.application_shortcuts.push(shortcut.clone()));
        }
        // TODO: register widget/window shortcuts
        self.shortcuts.push(shortcut);
        id
    }

    // TODO: remove_shortcut

    pub fn refresh_common_style(&mut self) {
        self.common_style = self.style().get_common(&self.style_element);
        self.size_hint_changed();
        self.update();
    }

    pub fn style_element(&self) -> &Element {
        &self.style_element
    }

    pub fn specific_style<T: ComputedElementStyle>(&self) -> Rc<T> {
        self.style().get(&self.style_element)
    }

    pub fn add_pseudo_class(&mut self, class: MyPseudoClass) {
        self.style_element.add_pseudo_class(class);
        self.refresh_common_style();
    }

    pub fn remove_pseudo_class(&mut self, class: MyPseudoClass) {
        self.style_element.remove_pseudo_class(class);
        self.refresh_common_style();
    }

    pub fn set_accessible(&mut self, value: bool) {
        if self.is_accessible == value {
            return;
        }
        self.is_accessible = value;
        self.update();
    }

    pub fn after_declare_children(&mut self, state: ChildrenUpdateState) {
        let previous = mem::take(&mut self.declared_children);
        let to_delete = previous.difference(&state.declared_children);
        for id in to_delete {
            if self.remove_child_by_id(*id).is_ok() {
                //                println!("deleted {:?}", id);
            }
        }
        self.declared_children = state.declared_children;
    }
}

#[derive(Debug)]
pub struct WidgetCommonTyped<T> {
    pub common: WidgetCommon,
    _marker: PhantomData<T>,
}

impl<W> WidgetCommonTyped<W> {
    pub fn id(&self) -> WidgetId<W> {
        WidgetId(self.common.id, PhantomData)
    }

    pub fn callback<E, F>(&self, func: F) -> Callback<E>
    where
        W: Widget,
        F: Fn(&mut W, E) -> Result<()> + 'static,
        E: 'static,
    {
        widget_callback(WidgetId::<W>::new(self.common.id), func)
    }

    pub fn add_child<T: Widget>(&mut self) -> &mut T {
        self.common.add_child::<T>()
    }
    pub fn add_child_with_key<T: Widget>(&mut self, key: impl Into<Key>) -> &mut T {
        self.common.add_child_with_key::<T>(key)
    }
}

impl<T> Deref for WidgetCommonTyped<T> {
    type Target = WidgetCommon;

    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl<T> DerefMut for WidgetCommonTyped<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.common
    }
}

impl<T> From<WidgetCommonTyped<T>> for WidgetCommon {
    fn from(value: WidgetCommonTyped<T>) -> Self {
        value.common
    }
}
