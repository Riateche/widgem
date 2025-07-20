use {
    super::{address, RawWidgetId, Widget, WidgetAddress, WidgetId, WidgetNotFound},
    crate::{
        callback::{widget_callback, Callback},
        event::Event,
        key::Key,
        layout::{LayoutItemOptions, SizeHints},
        shared_window::{SharedWindow, WindowId},
        shortcut::{Shortcut, ShortcutId, ShortcutScope},
        style::{
            common::CommonComputedStyle,
            css::{Element, PseudoClass},
            get_style,
        },
        system::{
            register_address, request_children_update, unregister_address, with_system,
            ChildrenUpdateState, ReportError,
        },
        types::{PhysicalPixels, Point, Rect, Size},
        widgets::WidgetExt,
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
    stringcase::kebab_case,
    winit::window::CursorIcon,
};

#[derive(Debug, Clone)]
pub struct WidgetCreationContext {
    pub parent_id: Option<RawWidgetId>,
    pub address: WidgetAddress,
    pub window: Option<SharedWindow>,
    pub parent_scale: f32,
    pub is_parent_enabled: bool,
    pub is_window_root: bool,
}

pub type EventFilterFn = dyn Fn(Event) -> Result<bool>;

/// Information about position, size and clipping of a widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetGeometry {
    /// Rect of this widget in parent coordinates.
    rect_in_parent: Rect,
    /// Top left of the parent widget in window coordinates.
    parent_top_left_in_window: Point,
    /// Parent widget's visible rect in parent widget's coordinates.
    parent_visible_rect_in_parent: Rect,
}

impl WidgetGeometry {
    pub fn root(size: Size) -> Self {
        WidgetGeometry {
            rect_in_parent: Rect::from_pos_size(Point::default(), size),
            parent_top_left_in_window: Point::default(),
            parent_visible_rect_in_parent: Rect::from_pos_size(Point::default(), size),
        }
    }

    /// Returns widget geometry of the child widget given the parent widget geometry and the
    /// rect of the child in the parent's coordinates.
    pub fn new(parent: &WidgetGeometry, rect_in_parent: Rect) -> Self {
        Self {
            rect_in_parent,
            parent_top_left_in_window: parent.rect_in_parent.top_left()
                + parent.parent_top_left_in_window,
            parent_visible_rect_in_parent: parent.visible_rect_in_self(),
        }
    }

    /// Rect of this widget in this widget's coordinates (top left is always zero).
    pub fn rect_in_self(&self) -> Rect {
        Rect::from_pos_size(Point::default(), self.rect_in_parent.size())
    }

    /// Rect of this widget in parent coordinates.
    pub fn rect_in_parent(&self) -> Rect {
        self.rect_in_parent
    }

    /// Size of the widget.
    pub fn size(&self) -> Size {
        self.rect_in_parent.size()
    }

    pub fn size_x(&self) -> PhysicalPixels {
        self.rect_in_parent.size_x()
    }

    pub fn size_y(&self) -> PhysicalPixels {
        self.rect_in_parent.size_y()
    }

    /// Rect of this widget in the window coordinates.
    pub fn rect_in_window(&self) -> Rect {
        self.rect_in_parent
            .translate(self.parent_top_left_in_window)
    }

    /// Visible rect of this widget in this widget's coordinates.
    pub fn visible_rect_in_self(&self) -> Rect {
        self.parent_visible_rect_in_parent
            .translate(-self.rect_in_parent.top_left())
            .intersect(self.rect_in_self())
    }
}

macro_rules! auto_bitflags {
    (
        $(#[$meta:meta])*
        $vis:vis struct $Name:ident : $T:ty { $( $flag:ident ),+ $(,)? }
    ) => {
        #[repr(u8)]
        #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
        enum __Flag { $( $flag ),+ }
        bitflags::bitflags! {
            $(#[$meta])*
            $vis struct $Name: $T {
                $(
                    const $flag = 1 << __Flag::$flag as $T;
                )+
            }
        }
    }
}

// Flag values are determined by their order.
// We don't expose these bit flags in the API, so
// changing the order of flags or inserting new flags is fine.
auto_bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct Flags: u64 {
        // explicitly set for this widget using `set_focusable`
        self_focusable,
        // reported by widget internally using `set_supports_focus`
        supports_focus,
        // widget currently has focus
        focused,
        // true if the widget is currently added to the window's list of focusable widgets
        registered_as_focusable,
        // whether IME is enabled when the widget has focus
        input_method_enabled,
        // If true, all mouse events from the parent propagate to this widget,
        // regardless of its boundaries.
        receives_all_mouse_events,
        // true if parent is enabled or if there is no parent
        parent_enabled,
        // true if this widget hasn't been explicitly disabled
        self_enabled,
        // true if this widget hasn't been explicitly hidden
        self_visible,
        // true if this widget is a window root widget (typically a `WindowWidget`)
        window_root,
        // true if the mouse pointer is currently over the widget
        under_mouse,
        // true if accessibility node hasn't been disabled for this widget
        accessibility_node_enabled,
        // true by default, but set to false if the widget
        // doesn't implement `handle_declare_children_request`
        has_declare_children_override,
    }
}

/// The first building block of a widget.
///
/// Any widget contains a `WidgetBase` object. You can obtain it by calling [base()](crate::widgets::Widget::base)
/// or [base_mut()](crate::widgets::Widget::base_mut). As a convention, any widget has a private field
/// `base: WidgetBaseOf<Self>` which dereferences to a `WidgetBase`.
///
/// `WidgetBase` stores some of the widget's state and handles some of the events dispatched to the widget.
/// It can be used to query or modify some common properties of a widget.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct WidgetBase {
    id: RawWidgetId,
    type_name: &'static str,
    flags: Flags,
    cursor_icon: CursorIcon,

    parent_id: Option<RawWidgetId>,
    address: WidgetAddress,
    window: Option<SharedWindow>,

    parent_scale: f32,
    self_scale: Option<f32>,

    // Present if the widget is not hidden, and only after layout.
    geometry: Option<WidgetGeometry>,

    #[derivative(Debug = "ignore")]
    pub children: BTreeMap<Key, Box<dyn Widget>>,
    pub layout_item_options: LayoutItemOptions,

    pub size_hint_x_cache: Option<SizeHints>,
    // TODO: limit count
    pub size_hint_y_cache: HashMap<PhysicalPixels, SizeHints>,

    // TODO: multiple filters?
    // TODO: accept/reject event from filter; option to run filter after on_event
    #[derivative(Debug = "ignore")]
    pub event_filter: Option<Box<EventFilterFn>>,

    pub shortcuts: Vec<Shortcut>,
    pub style_element: Element,
    pub common_style: Rc<CommonComputedStyle>,

    pub num_added_children: u32,
    // Direct and indirect children created by last call of this widget's
    // `handle_update_children`.
    pub declared_children: HashSet<RawWidgetId>,
}

impl Drop for WidgetBase {
    fn drop(&mut self) {
        unregister_address(self.id);
        // Drop and unmount children before unmounting self.
        self.children.clear();
        self.remove_accessibility_node();
        for shortcut in &self.shortcuts {
            // TODO: deregister widget/window shortcuts
            if shortcut.scope == ShortcutScope::Application {
                with_system(|system| system.application_shortcuts.retain(|s| s.id != shortcut.id));
            }
        }
    }
}

fn last_path_part(str: &str) -> &str {
    str.rsplit("::")
        .next()
        .expect("rsplit always returns at least one element")
}

impl WidgetBase {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<T: Widget>(ctx: WidgetCreationContext) -> WidgetBaseOf<T> {
        let id = ctx.address.widget_id();
        register_address(id, ctx.address.clone());

        let type_name = T::type_name();
        let style_element = Element::new(kebab_case(last_path_part(type_name)))
            .with_pseudo_class(PseudoClass::Enabled);
        let common_style = get_style(&style_element, ctx.parent_scale);
        let mut common = Self {
            id,
            type_name,
            flags: Flags::self_enabled
                | Flags::self_visible
                | Flags::accessibility_node_enabled
                | Flags::has_declare_children_override
                | if ctx.is_parent_enabled {
                    Flags::parent_enabled
                } else {
                    Flags::empty()
                }
                | if ctx.is_window_root {
                    Flags::window_root
                } else {
                    Flags::empty()
                },
            parent_id: ctx.parent_id,
            address: ctx.address,
            window: ctx.window,
            parent_scale: ctx.parent_scale,
            self_scale: None,
            geometry: None,
            cursor_icon: CursorIcon::Default,
            children: BTreeMap::new(),
            layout_item_options: LayoutItemOptions::default(),
            size_hint_x_cache: None,
            size_hint_y_cache: HashMap::new(),
            event_filter: None,
            shortcuts: Vec::new(),
            style_element,
            common_style,
            num_added_children: 0,
            declared_children: Default::default(),
        };

        if let Some(window) = &common.window {
            let root_widget_id = window.root_widget_id();
            window.remove_accessibility_node(
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
                        warn!("WidgetBase::new: empty address encountered");
                        "".into()
                    }),
            );
        }
        common.update();
        common.enabled_changed();
        common.focusable_changed();
        common.refresh_common_style();

        WidgetBaseOf {
            base: common,
            _marker: PhantomData,
        }
    }

    // TODO: revise behavior on hidden windows

    /// True if this widget is currently visible.
    ///
    /// Widgets are visible by default, but can become hidden in a variety of ways:
    /// - You can explicitly hide a widget with [`set_visible(false)`](Self::set_visible).
    /// - A widget is not visible if its parent is not visible.
    /// - The parent can choose to hide any of its direct children when calculating its layout.
    ///   For example, a tab widget would hide widgets corresponding to the contents of inactive tabs.
    /// - A widget is not visible if it's out of bounds of its direct or indirect parents.
    ///   This commonly occurs to widgets inside a [crate::widgets::scroll_area::ScrollArea].
    /// - A widget is not visible if it doesn't belong to a window.
    ///
    /// The following cases do not count as hidden widgets:
    /// - A widget obscured by another widget positioned on top of it.
    /// - A widget obscued by another OS window.
    /// - A widget in a minimized or hidden window.
    pub fn is_visible(&self) -> bool {
        self.geometry
            .as_ref()
            .is_some_and(|g| !g.visible_rect_in_self().is_empty())
    }

    /// True if this widget hasn't been explicitly hidden.
    ///
    /// This method can be used to tell if the widget is hidden because `set_visible(false)` was called on it
    /// or because of its parent. In most cases it's sufficient to use [is_visible](Self::is_visible) instead.
    pub fn is_self_visible(&self) -> bool {
        self.flags.contains(Flags::self_visible)
    }

    /// Hide or show a widget.
    ///
    /// A widget hidden with `set_visible(false)` will never be automatically shown. It can only be shown with
    /// `set_visible(true)`.
    ///
    /// A widget can also be hidden because of its parent or its position within a parent
    /// (see [is_visible](Self::is_visible)).
    /// If this is the case, calling `set_visible` will still change the visibility flag of the widget, but
    /// the widget will not become visible unless all conditions for its visibility are met.
    pub fn set_visible(&mut self, value: bool) -> &mut Self {
        if self.is_self_visible() == value {
            return self;
        }
        self.flags.set(Flags::self_visible, value);
        self.size_hint_changed(); // trigger layout
        self
    }

    /// True if this widget is a root widget of an OS window.
    ///
    /// This is true for [crate::widgets::Window] and false for all other provided widget types.
    pub fn is_window_root(&self) -> bool {
        self.flags.contains(Flags::window_root)
    }

    /// True if this widget participates in a grid layout.
    ///
    /// This is true if all the following conditions hold:
    /// - It's not a [window root](crate::widgets::Window).
    /// - It hasn't been explicitly hidden with [`set_visible(false)`](Self::set_visible).
    /// - It has the row and the column set.
    pub(crate) fn is_in_grid(&self) -> bool {
        self.layout_item_options.is_in_grid() && !self.is_window_root() && self.is_self_visible()
    }

    /// True if this widget hasn't been explicitly disabled.
    ///
    /// This method can be used to tell if the widget is disabled because `set_enabled(false)` was called on it
    /// or because its parent is disabled. In most cases it's sufficient to use [is_enabled](Self::is_enabled) instead.
    pub fn is_self_enabled(&self) -> bool {
        self.flags.contains(Flags::self_enabled)
    }

    pub(crate) fn is_parent_enabled(&self) -> bool {
        self.flags.contains(Flags::parent_enabled)
    }

    /// True if this widget is enabled.
    ///
    /// Disabled widgets do not receive input events and have an alternate (usually grayed out) appearance.
    /// Use [set_enabled](WidgetExt::set_enabled) to enable or disable a widget. If a widget is disabled,
    /// all its children are disabled as well.
    pub fn is_enabled(&self) -> bool {
        self.flags
            .contains(Flags::self_enabled | Flags::parent_enabled)
    }

    pub(crate) fn self_enabled_changed(&mut self, enabled: bool) -> &mut Self {
        if self.flags.contains(Flags::self_enabled) == enabled {
            return self;
        }
        let old_enabled = self.is_enabled();
        self.flags.set(Flags::self_enabled, enabled);
        let new_enabled = self.is_enabled();
        if old_enabled == new_enabled {
            return self;
        }
        self.enabled_changed();
        self
    }

    pub(crate) fn parent_enabled_changed(&mut self, enabled: bool) -> &mut Self {
        if self.flags.contains(Flags::parent_enabled) == enabled {
            return self;
        }
        let old_enabled = self.is_enabled();
        self.flags.set(Flags::parent_enabled, enabled);
        let new_enabled = self.is_enabled();
        if old_enabled == new_enabled {
            return self;
        }
        self.enabled_changed();
        self
    }

    /// Returns `true` if this widget can receive focus.
    ///
    /// The widget is focusable if it supports focus, is not disabled and hasn't been configured with `set_focusable(false)`.
    /// Widgets that support focus are focusable by default.
    pub fn is_focusable(&self) -> bool {
        self.flags
            .contains(Flags::self_focusable | Flags::supports_focus)
            && self.is_enabled()
    }

    /// Returns `true` if the widget currently has focus.
    pub fn is_focused(&self) -> bool {
        self.flags.contains(Flags::focused) && self.is_enabled()
    }

    /// Returns `true` if the widget's OS window is focused.
    pub fn is_window_focused(&self) -> bool {
        self.window.as_ref().is_some_and(|w| w.is_focused())
    }

    pub(crate) fn has_declare_children_override(&self) -> bool {
        self.flags.contains(Flags::has_declare_children_override)
    }

    pub(crate) fn set_has_declare_children_override(&mut self, value: bool) -> &mut Self {
        self.flags.set(Flags::has_declare_children_override, value);
        self
    }

    pub fn new_creation_context(
        &self,
        new_id: RawWidgetId,
        key: Key,
        root_of_window: Option<SharedWindow>,
    ) -> WidgetCreationContext {
        WidgetCreationContext {
            parent_id: Some(self.id),
            address: self.address.clone().join(key, new_id),
            is_window_root: root_of_window.is_some(),
            window: root_of_window.or_else(|| self.window.clone()),
            parent_scale: self.scale(),
            is_parent_enabled: self.is_enabled(),
        }
    }

    // Request redraw and accessibility update
    pub fn update(&mut self) {
        if let Some(window) = &self.window {
            window.request_redraw();
            window.request_accessibility_update(self.address.clone());
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
                        state.declared_children.insert(old_widget.base().id);
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

        let new_id = new_id.unwrap_or_else(RawWidgetId::new_unique);
        let ctx = if T::is_window_root_type() {
            let new_window = SharedWindow::new(new_id);
            self.new_creation_context(new_id, key.clone(), Some(new_window.clone()))
        } else {
            self.new_creation_context(new_id, key.clone(), None)
        };
        // This may delete the old widget.
        self.children
            .insert(key.clone(), Box::new(T::new(WidgetBase::new::<T>(ctx))));
        self.size_hint_changed();
        let widget = self.children.get_mut(&key).unwrap();
        if declare {
            with_system(|system| {
                if let Some(state) = &mut system.current_children_update {
                    state.declared_children.insert(widget.base().id);
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

    pub fn set_layout_item_options(&mut self, options: LayoutItemOptions) -> &mut Self {
        if self.layout_item_options == options {
            return self;
        }
        self.layout_item_options = options;
        self.size_hint_changed();
        self
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
            if current_widget.base().id == parent_id {
                return current_widget
                    .base_mut()
                    .remove_child(&address.path.last().unwrap().0);
            }
            current_widget = current_widget
                .base_mut()
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

    /// Returns a shared handle to a window that contains this widget.
    ///
    /// A widget is considered to be in a window if it's a [`Window`](crate::widgets::Window)
    /// or any of its direct or indirect parts is a `Window`. [`RootWidget`](crate::widgets::RootWidget)
    /// and its direct non-`Window` children are not associated with any window.
    ///
    /// See also: [`window_or_err`](Self::window_or_err), [`window_id`](Self::window_id).
    pub fn window(&self) -> Option<&SharedWindow> {
        self.window.as_ref()
    }

    /// Returns a shared handle to a window that contains this widget, or an error if it's not associated with a window.
    ///
    /// See [`window`](Self::window) for more information.
    ///
    /// Use this function for a more convenient early exit with `?`. There are many cases when it makes sense
    /// to assume that there is a window, for example, when handling window events or if the widget is
    /// intended to be used in a window (which is true for most visual widgets).
    pub fn window_or_err(&self) -> Result<&SharedWindow> {
        self.window.as_ref().context("no window")
    }

    /// Returns ID of the window associated with this widget.
    ///
    /// See [`window`](Self::window) for more information.
    pub fn window_id(&self) -> Option<WindowId> {
        self.window.as_ref().map(|w| w.id())
    }

    /// Returns the *address* of the widget.
    ///
    /// The address is the path from the root widget to this widget.
    /// The address can be used to quickly access the widget from the root widget
    /// or from any other indirect parent of the widget.
    ///
    /// The address of a widget cannot change. The only way to "change" it is to
    /// delete the widget and recreate it at another address.
    pub fn address(&self) -> &WidgetAddress {
        &self.address
    }

    // private: we need to use `WidgetExt::set_geometry` to dispatch the event.
    pub(crate) fn set_geometry(&mut self, value: Option<WidgetGeometry>) {
        self.geometry = value;
    }

    /// Returns information about current position, size and visible rect of a widget.
    ///
    /// Geometry is `None` if the widget is hidden, if it's positioned completely out of bounds
    /// of its parent, or if the parent widget hasn't updated its layout yet.
    pub fn geometry(&self) -> Option<&WidgetGeometry> {
        self.geometry.as_ref()
    }

    /// Returns information about current position, size and visible rect of a widget,
    /// or an error if there is no geometry.
    ///
    /// See also: [geometry](Self::geometry).
    pub fn geometry_or_err(&self) -> Result<&WidgetGeometry> {
        self.geometry.as_ref().context("no geometry")
    }

    /// Returns current size of the widget.
    ///
    /// See also: [geometry](Self::geometry).
    pub fn size(&self) -> Option<Size> {
        self.geometry.as_ref().map(|g| g.size())
    }

    /// Returns current size of the widget,
    /// or an error if there is no geometry.
    ///
    /// See also: [geometry](Self::geometry).
    pub fn size_or_err(&self) -> Result<Size> {
        Ok(self.geometry_or_err()?.size())
    }

    /// Returns current boundary of the widget in the window coordinates.
    ///
    /// See also: [geometry](Self::geometry).
    pub fn rect_in_window(&self) -> Option<Rect> {
        self.geometry.as_ref().map(|g| g.rect_in_window())
    }

    /// Returns current boundary of the widget in the window coordinates,
    /// or an error if there is no geometry.
    ///
    /// See also: [geometry](Self::geometry).
    pub fn rect_in_window_or_err(&self) -> Result<Rect> {
        Ok(self.geometry_or_err()?.rect_in_window())
    }

    /// Returns current boundary of the widget in the parent widget's coordinates.
    ///
    /// See also: [geometry](Self::geometry).
    pub fn rect_in_parent(&self) -> Option<Rect> {
        self.geometry.as_ref().map(|g| g.rect_in_parent)
    }

    /// Returns current boundary of the widget in the parent widget's coordinates,
    /// or an error if there is no geometry.
    ///
    /// See also: [geometry](Self::geometry).
    pub fn rect_in_parent_or_err(&self) -> Result<Rect> {
        self.geometry_or_err().map(|g| g.rect_in_parent)
    }

    /// Returns current visible rectangle of the widget in the widget's coordinates.
    ///
    /// See also: [geometry](Self::geometry).
    pub fn visible_rect_in_self(&self) -> Option<Rect> {
        self.geometry.as_ref().map(|g| g.visible_rect_in_self())
    }

    /// Returns current visible rectangle of the widget in the widget's coordinates,
    /// or an error if there is no geometry.
    ///
    /// See also: [geometry](Self::geometry).
    pub fn visible_rect_in_self_or_err(&self) -> Result<Rect> {
        self.geometry_or_err().map(|g| g.visible_rect_in_self())
    }

    /// Returns the boundary of the widget in the widget's coordinates (top left is always zero).
    ///
    /// See also: [geometry](Self::geometry).
    pub fn rect_in_self(&self) -> Option<Rect> {
        self.geometry.as_ref().map(|g| g.rect_in_self())
    }

    /// Returns the boundary of the widget in the widget's coordinates (top left is always zero),
    /// or an error if there is no geometry.
    ///
    /// See also: [geometry](Self::geometry).
    pub fn rect_in_self_or_err(&self) -> Result<Rect> {
        Ok(self.geometry_or_err()?.rect_in_self())
    }

    /// Processes the event before it's dispatched to the widget.
    ///
    /// Returns `true` if the event is consumed and shouldn't be dispatched to the widget.
    pub(crate) fn before_event(&mut self, event: &Event) -> bool {
        match &event {
            Event::FocusIn(_) => {
                self.flags.insert(Flags::focused);
            }
            Event::FocusOut(_) => {
                self.flags.remove(Flags::focused);
            }
            Event::WindowFocusChange(_) => {}
            Event::MouseInput(event) => {
                for child in self.children.values_mut().rev() {
                    if let Some(rect_in_parent) = child.base().rect_in_parent() {
                        if let Some(child_event) = event
                            .map_to_child(rect_in_parent, child.base().receives_all_mouse_events())
                        {
                            if child.dispatch(child_event.into()) {
                                return true;
                            }
                        }
                    }
                }
            }
            Event::MouseScroll(event) => {
                for child in self.children.values_mut().rev() {
                    if let Some(rect_in_parent) = child.base().rect_in_parent() {
                        if let Some(child_event) = event
                            .map_to_child(rect_in_parent, child.base().receives_all_mouse_events())
                        {
                            if child.dispatch(child_event.into()) {
                                return true;
                            }
                        }
                    }
                }
            }
            Event::MouseMove(event) => {
                for child in self.children.values_mut().rev() {
                    if let Some(rect_in_parent) = child.base().rect_in_parent() {
                        if let Some(child_event) = event
                            .map_to_child(rect_in_parent, child.base().receives_all_mouse_events())
                        {
                            if child.dispatch(child_event.into()) {
                                return true;
                            }
                        }
                    }
                }
            }
            Event::MouseEnter(_) => {
                self.flags.insert(Flags::under_mouse);
            }
            Event::MouseLeave(_) => {
                self.flags.remove(Flags::under_mouse);
            }
            Event::StyleChange(_) => {
                self.refresh_common_style();
            }
            Event::Draw(event) => {
                let Some(size) = self.size_or_err().or_report_err() else {
                    return false;
                };

                event.stroke_and_fill_rounded_rect(
                    Rect::from_pos_size(Point::default(), size),
                    &self.common_style.border,
                    self.common_style.background.as_ref(),
                );
            }
            Event::KeyboardInput(_)
            | Event::InputMethod(_)
            | Event::Layout(_)
            | Event::AccessibilityAction(_) => {}
        }
        false
    }

    fn focusable_changed(&mut self) {
        let is_focusable = self.is_focusable();
        if is_focusable != self.flags.contains(Flags::registered_as_focusable) {
            if let Some(window) = &self.window {
                if is_focusable {
                    window.add_focusable_widget(self.address.clone(), self.id);
                } else {
                    window.remove_focusable_widget(self.address.clone(), self.id);
                }
                self.flags.set(Flags::registered_as_focusable, is_focusable);
            } else {
                self.flags.set(Flags::registered_as_focusable, false);
            }
        }
    }

    fn enabled_changed(&mut self) {
        self.focusable_changed();
        // TODO: widget should receive MouseLeave even if it's disabled
        let is_enabled = self.is_enabled();
        for child in self.children.values_mut() {
            child.set_parent_enabled(is_enabled);
        }
    }

    /// Enable or disable input method for this widget.
    ///
    /// IME will be enabled for the window if the currently focused widget has input method enabled.
    /// Default value is false.
    ///
    /// Call this function with `true` in your widget's `new` function if your widget receives text input
    /// (as opposed to e.g. hotkeys). Implement [handle_input_method](Widget::handle_input_method) to
    /// handle events from the input method.
    pub fn set_input_method_enabled(&mut self, enabled: bool) -> &mut Self {
        if self.flags.contains(Flags::input_method_enabled) == enabled {
            return self;
        }
        self.flags.set(Flags::input_method_enabled, enabled);
        if self.is_focused() {
            if let Some(window) = &self.window {
                window.set_ime_allowed(enabled);
            }
        }
        self
    }

    /// Returns true if input method is enabled for this widget.
    ///
    /// See also [set_input_method_enabled](Self::set_input_method_enabled).
    pub fn is_input_method_enabled(&self) -> bool {
        self.flags.contains(Flags::input_method_enabled)
    }
    // TODO: fix/test: widget should receive mouseenter event and under_mouse flag
    // even if its child obscures the area (like in html)

    /// Check if the mouse is over this widget.
    ///
    /// If the widget becomes a *mouse grabber*, it will be considered under mouse until
    /// all mouse buttons are released, even if the mouse moves out of the widget's boundary.
    pub fn is_under_mouse(&self) -> bool {
        self.flags.contains(Flags::under_mouse)
    }

    fn remove_accessibility_node(&mut self) {
        if let Some(window) = &self.window {
            let root_widget_id = window.root_widget_id();
            window.update_accessibility_node(
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
        let w = self.widget_raw(id.raw())?;
        Ok(w.downcast_mut::<W>().expect("widget downcast failed"))
    }

    pub fn widget_raw(&mut self, id: RawWidgetId) -> Result<&mut dyn Widget, WidgetNotFound> {
        // TODO: speed up
        for child in self.children.values_mut() {
            if child.base().id == id {
                return Ok(child.as_mut());
            }
            if let Ok(widget) = child.base_mut().widget_raw(id) {
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
        self.common_style = get_style(&self.style_element, self.scale());
        self.size_hint_changed();
        self.update();
    }

    pub fn style_element(&self) -> &Element {
        &self.style_element
    }

    /// Scale of the widget.
    ///
    /// Scale is the multiplication factor that's used when measurements in style definitions
    /// ([crate::types::LogicalPixels]) are converted to pixel measurements used for
    /// layout, drawing, and event processing ([crate::types::PhysicalPixels]).
    ///
    /// By default, scale is determined by the scale factor reported by the OS for the OS window
    /// that contains the widget. Scale may be overriden with [`App::set_scale`](crate::App::with_scale),
    /// in which case the OS scale is ignored.
    ///
    /// Scale can be changed manually for any widget
    /// with [`WidgetExt::set_scale`](crate::widgets::WidgetExt::set_scale).
    /// This value propagates to all child widgets.
    pub fn scale(&self) -> f32 {
        self.self_scale.unwrap_or(self.parent_scale)
    }

    // Intentionally private: we need to use `WidgetExt::set_scale` to dispatch the event properly.
    pub(crate) fn set_scale(&mut self, scale: Option<f32>) -> &mut Self {
        self.self_scale = scale;
        self
    }

    /// Scale of the parent widget.
    ///
    /// For the root widget, it returns the default scale.
    pub fn parent_scale(&self) -> f32 {
        self.parent_scale
    }

    /// Returns the value set with
    /// [`WidgetExt::set_scale`](crate::widgets::WidgetExt::set_scale), if any.
    ///
    /// It returns `None` if `set_scale` hasn't been called for this widget.
    /// This value doesn't depend on the parent scale. In most cases, [scale](Self::scale) is more suitable,
    /// as that one always returns the current effective scale of the widget.
    pub fn self_scale(&self) -> Option<f32> {
        self.self_scale
    }

    /// Check if the accessibility node hasn't been disabled for this widget.
    ///
    /// This value corresponds to the value set by [`set_accessibility_node_enabled`](Self::set_accessibility_node_enabled).
    /// Default is true for every widget. Note that some widgets may not support accessibility nodes,
    /// and it doesn't affect the return value of `is_accessibility_node_enabled`.
    pub fn is_accessibility_node_enabled(&self) -> bool {
        self.flags.contains(Flags::accessibility_node_enabled)
    }

    /// Turn the accessibility node of this widget on or off.
    ///
    /// Accessibility nodes allow users to interact with a widget using screen readers or other assistive technologies.
    /// Accessibility nodes are enabled by default. They should only be disabled if the widget
    /// is redundant and the same functionality is already available through other widgets.
    ///
    /// This function does nothing in widgets that don't implement an accessibility node.
    ///
    /// This setting doesn't propagate to child widgets.
    pub fn set_accessibility_node_enabled(&mut self, value: bool) -> &mut Self {
        if self.flags.contains(Flags::accessibility_node_enabled) == value {
            return self;
        }
        self.flags.set(Flags::accessibility_node_enabled, value);
        self.update();
        self
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

    /// Returns untyped ID of this widget.
    ///
    /// To obtain a typed ID of the widget, use [WidgetExt::id] externally
    /// or `self.common.id()` internally.
    pub fn id(&self) -> RawWidgetId {
        self.id
    }

    /// Returns full path to the widget type as a string.
    ///
    /// Returns the same value as [Widget::type_name].
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Returns `true` if this widget's implementation is able to receive focus.
    ///
    /// This value is set by the widget's implementation. Widgets cannot be focusable if they don't support focus.
    /// Most widgets that support focus are focusable by default, but can be made unfocusable using [set_focusable](Self::set_focusable).
    pub fn supports_focus(&self) -> bool {
        self.flags.contains(Flags::supports_focus)
    }

    /// Set focusability of the widget.
    ///
    /// Widgets that support focus are usually focusable by default, so usually you shouldn't use this function.
    /// Disabling focus can make your interface less accessible.
    pub fn set_focusable(&mut self, focusable: bool) -> &mut Self {
        if focusable == self.flags.contains(Flags::self_focusable) {
            return self;
        }
        if focusable && !self.supports_focus() {
            warn!("cannot do `set_focusable(true)` on a widget that doesn't support focus");
            return self;
        }
        self.flags.set(Flags::self_focusable, focusable);
        self.focusable_changed();
        self
    }

    /// If true, all mouse events from the parent propagate to this widget,
    /// regardless of its boundaries.
    pub fn set_receives_all_mouse_events(&mut self, enabled: bool) -> &mut Self {
        if self.flags.contains(Flags::receives_all_mouse_events) == enabled {
            return self;
        }
        self.flags.set(Flags::receives_all_mouse_events, enabled);
        self
    }

    /// If true, all mouse events from the parent propagate to this widget,
    /// regardless of its boundaries.
    pub fn receives_all_mouse_events(&self) -> bool {
        self.flags.contains(Flags::receives_all_mouse_events)
    }

    /// Returns the shape of the mouse pointer when it's over this widget.
    ///
    /// Returns the value that was previously set with [`set_cursor_icon`](Self::set_cursor_icon).
    pub fn cursor_icon(&self) -> CursorIcon {
        self.cursor_icon
    }

    /// Configure the shape of the mouse pointer when it's over this widget.
    ///
    /// Default value is [CursorIcon::Default].
    pub fn set_cursor_icon(&mut self, cursor_icon: CursorIcon) -> &mut Self {
        self.cursor_icon = cursor_icon;
        self
    }

    /// ID of the parent widget.
    ///
    /// The parent widget is the owner of its direct children.
    /// For UI widgets, the parent widget is a container of its child widgets.
    /// Child widgets are always positioned relative to their parent.
    /// Children can only be visplayed in the boundary of their parent.
    ///
    /// The parent ID of the widget cannot be changed.
    ///
    /// Returns `None` for [crate::widgets::root::RootWidget].
    pub fn parent_id(&self) -> Option<RawWidgetId> {
        self.parent_id
    }
}

#[derive(Debug)]
pub struct WidgetBaseOf<T> {
    base: WidgetBase,
    _marker: PhantomData<T>,
}

impl<W> WidgetBaseOf<W> {
    pub fn untyped(&self) -> &WidgetBase {
        &self.base
    }

    pub fn untyped_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    pub fn id(&self) -> WidgetId<W> {
        WidgetId::new(self.base.id)
    }

    pub fn callback<E, F>(&self, func: F) -> Callback<E>
    where
        W: Widget,
        F: Fn(&mut W, E) -> Result<()> + 'static,
        E: 'static,
    {
        widget_callback(WidgetId::<W>::new(self.base.id), func)
    }

    pub fn add_child<T: Widget>(&mut self) -> &mut T {
        self.base.add_child::<T>()
    }
    pub fn add_child_with_key<T: Widget>(&mut self, key: impl Into<Key>) -> &mut T {
        self.base.add_child_with_key::<T>(key)
    }

    /// Report the widget as supporting (or not supporting) focus.
    ///
    /// This function should only be called by the widget itself and should never be called by other widgets.
    /// To control focusability of other widgets, use [set_focusable](WidgetBase::set_focusable) instead.
    ///
    /// Call `set_supports_focus(true)` from the `new` function of your widget if it supports being focusable.
    /// In most cases it's not needed to ever call it again.
    ///
    /// Note that it also implies `set_focusable(true)`. In a rare case when you don't want your widget
    /// to be focusable by default,
    /// Call `set_focusable(false)` **after** `set_supports_focus(true)`.
    pub fn set_supports_focus(&mut self, supports_focus: bool) -> &mut Self {
        let old_flags = self.base.flags.bits();
        self.base.flags.set(
            Flags::supports_focus | Flags::self_focusable,
            supports_focus,
        );
        if self.base.flags.bits() == old_flags {
            return self;
        }
        self.focusable_changed();
        self
    }
}

impl<T> Deref for WidgetBaseOf<T> {
    type Target = WidgetBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T> DerefMut for WidgetBaseOf<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<T> From<WidgetBaseOf<T>> for WidgetBase {
    fn from(value: WidgetBaseOf<T>) -> Self {
        value.base
    }
}
