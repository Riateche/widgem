use {
    super::{RawWidgetId, Widget, WidgetAddress, WidgetId, WidgetNotFound},
    crate::{
        callback::Callback,
        child_key::ChildKey,
        event::{Event, FocusReason},
        items::{
            with_index::{Items, ItemsMut},
            with_key::{ItemsWithKey, ItemsWithKeyMut},
        },
        layout::{Layout, LayoutItemOptions, SizeHint},
        shared_window::{SharedWindow, WindowId},
        shortcut::{Shortcut, ShortcutId, ShortcutScope},
        style::{
            common::{BaseComputedStyle, ComputedElementStyle},
            css::{PseudoClass, StyleSelector},
            load_css,
        },
        system::ReportError,
        types::{PhysicalPixels, Point, Rect, Size},
        widget_initializer::WidgetInitializer,
        widgets::WidgetExt,
        App,
    },
    anyhow::{Context, Result},
    derivative::Derivative,
    itertools::Itertools,
    lightningcss::stylesheet::StyleSheet,
    std::{
        cell::RefCell,
        collections::{BTreeMap, HashMap, HashSet},
        fmt::Debug,
        marker::PhantomData,
        ops::{Deref, DerefMut},
        rc::Rc,
    },
    tracing::{error, warn},
    winit::window::CursorIcon,
};

fn main_key() -> ChildKey {
    "__main".into()
}

fn default_scale(app: &App) -> f32 {
    if let Some(scale) = app.config().fixed_scale {
        return scale;
    }
    let monitor = app
        .primary_monitor()
        .or_else(|| app.available_monitors().next());
    if let Some(monitor) = monitor {
        monitor.scale_factor() as f32
    } else {
        warn!("unable to find any monitors");
        1.0
    }
}

#[derive(Debug)]
struct WidgetCreationContext {
    parent_id: Option<RawWidgetId>,
    address: WidgetAddress,
    window: Option<SharedWindow>,
    app: App,
    parent_scale: f32,
    is_parent_enabled: bool,
    is_window_root: bool,
}

pub type EventFilterFn = dyn FnMut(Event) -> Result<bool>;

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

#[derive(Debug, Default)]
struct Cache {
    size_hint_x: HashMap<Option<PhysicalPixels>, SizeHint>,
    // TODO: limit count
    size_hint_y: HashMap<PhysicalPixels, SizeHint>,
}

#[derive(Debug)]
struct CustomStyle {
    code: String,
    style_sheet: StyleSheet<'static, 'static>,
}

/// The first building block of a widget.
///
/// Any widget contains a `WidgetBase` object. You can obtain it by calling [base()](crate::widgets::Widget::base)
/// or [base_mut()](crate::widgets::Widget::base_mut). As a convention, any widget has a private field
/// <code>base: [WidgetBaseOf]&lt;Self&gt;</code> which dereferences to a `WidgetBase`.
///
/// `WidgetBase` stores some of the widget's state and handles some of the events dispatched to the widget.
/// It can be used to query or modify some common properties of a widget.
///
/// See also: [WidgetExt] trait that provides more actions on any widget.
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
    app: App,

    parent_scale: f32,
    self_scale: Option<f32>,

    // Present if the widget is not hidden, and only after layout.
    geometry: Option<WidgetGeometry>,

    layout: Layout,

    #[derivative(Debug = "ignore")]
    children: BTreeMap<ChildKey, Box<dyn Widget>>,
    layout_item_options: LayoutItemOptions,

    // TODO: option to run event filter after on_event
    // TODO: index map for deterministic order?

    // key is the widget that registered the event filter
    #[derivative(Debug = "ignore")]
    event_filters: HashMap<RawWidgetId, Box<EventFilterFn>>,

    shortcuts: HashMap<ShortcutId, Shortcut>,
    style_selector: StyleSelector,
    base_style: Rc<BaseComputedStyle>,
    style: Option<CustomStyle>,

    cache: RefCell<Cache>,
}

impl Drop for WidgetBase {
    fn drop(&mut self) {
        // Drop and unmount children before unmounting self.
        self.children.clear();
        self.remove_accessibility_node();
        self.app.unregister_address(self.id);
        for shortcut in self.shortcuts.values() {
            // TODO: deregister widget/window shortcuts
            if shortcut.scope == ShortcutScope::Application {
                self.app.remove_shortcut(shortcut.id);
            }
        }
        if self.is_window_root() {
            if let Some(window) = self.window.take() {
                self.app.remove_window(&window);
                if !window.try_remove() {
                    error!("window root widget has been deleted, but there are remaining references to SharedWindow");
                }
            } else {
                error!("missing window for a window root widget");
            }
        }
    }
}

fn last_path_part(str: &str) -> &str {
    str.rsplit("::")
        .next()
        .expect("rsplit always returns at least one element")
}

fn get_computed_style<T: ComputedElementStyle>(
    app: &App,
    element: &StyleSelector,
    scale: f32,
    custom_style: Option<&CustomStyle>,
) -> Rc<T> {
    app.style()
        .get(element, scale, custom_style.map(|s| &s.style_sheet))
}

// Various private impls.
impl WidgetBase {
    pub(crate) fn new_root<T: Widget>(app: App) -> WidgetBaseOf<T> {
        let id = RawWidgetId::new_unique();
        Self::new(WidgetCreationContext {
            parent_id: None,
            address: WidgetAddress::root(id),
            window: None,
            app,
            // Scale doesn't matter for root widget. Window will set scale for its content.
            parent_scale: 1.0,
            is_parent_enabled: true,
            is_window_root: false,
        })
    }

    #[allow(clippy::new_ret_no_self)]
    fn new<T: Widget>(ctx: WidgetCreationContext) -> WidgetBaseOf<T> {
        let id = ctx.address.widget_id();
        ctx.app.register_address(id, ctx.address.clone());

        let type_name = T::type_name();
        let style_selector = StyleSelector::new(last_path_part(type_name).into())
            .with_pseudo_class(PseudoClass::Enabled);
        let self_scale = if ctx.is_window_root {
            Some(default_scale(&ctx.app))
        } else {
            None
        };
        let common_style = get_computed_style(
            &ctx.app,
            &style_selector,
            self_scale.unwrap_or(ctx.parent_scale),
            None,
        );
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
            app: ctx.app,
            parent_scale: ctx.parent_scale,
            self_scale,
            geometry: None,
            cursor_icon: CursorIcon::Default,
            children: BTreeMap::new(),
            layout_item_options: LayoutItemOptions::default(),
            event_filters: HashMap::new(),
            shortcuts: HashMap::new(),
            style_selector,
            style: None,
            base_style: common_style,
            layout: Layout::default(),
            cache: RefCell::new(Cache::default()),
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

    pub(crate) fn has_declare_children_override(&self) -> bool {
        self.flags.contains(Flags::has_declare_children_override)
    }

    pub(crate) fn set_has_declare_children_override(&mut self, value: bool) -> &mut Self {
        self.flags.set(Flags::has_declare_children_override, value);
        self
    }

    fn new_creation_context(
        &self,
        new_id: RawWidgetId,
        key: ChildKey,
        root_of_window: Option<SharedWindow>,
    ) -> WidgetCreationContext {
        WidgetCreationContext {
            parent_id: Some(self.id),
            address: self.address.clone().join(key, new_id),
            is_window_root: root_of_window.is_some(),
            window: root_of_window.or_else(|| self.window.clone()),
            app: self.app.create_app_handle(),
            parent_scale: self.scale(),
            is_parent_enabled: self.is_enabled(),
        }
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
                    &self.base_style.border,
                    self.base_style.background.as_ref(),
                );
            }
            Event::KeyboardInput(_)
            | Event::InputMethod(_)
            | Event::Layout(_)
            | Event::AccessibilityAction(_) => {}
        }

        for event_filter in self.event_filters.values_mut() {
            let accepted = event_filter(event.clone()).or_report_err().unwrap_or(false);
            if accepted {
                return true;
            }
        }

        false
    }

    /// Returns information about the widget's common style properties.
    pub(crate) fn base_style(&self) -> &BaseComputedStyle {
        &self.base_style
    }
}

/// <h2>Widget properties</h2>
impl WidgetBase {
    /// Returns untyped ID of this widget.
    ///
    /// To obtain a typed ID of the widget, use [WidgetExt::id] externally
    /// or `self.base.id()` internally.
    pub fn id(&self) -> RawWidgetId {
        self.id
    }

    /// Returns full path to the widget type as a string.
    ///
    /// Returns the same value as [Widget::type_name].
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Returns the address of the widget.
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

    /// Returns a shared handle to a window that contains this widget.
    ///
    /// A widget is considered to be in a window if it's a [Window](crate::Window)
    /// or any of its direct or indirect parts is a `Window`. [RootWidget](crate::widgets::RootWidget)
    /// and its direct non-`Window` children are not associated with any window.
    ///
    /// See also: [window_or_err](Self::window_or_err), [window_id](Self::window_id).
    pub fn window(&self) -> Option<&SharedWindow> {
        self.window.as_ref()
    }

    /// Returns a shared handle to a window that contains this widget, or an error if it's not associated with a window.
    ///
    /// See [window](Self::window) for more information.
    ///
    /// Use this function for a more convenient early exit with `?`. There are many cases when it makes sense
    /// to assume that there is a window, for example, when handling window events or if the widget is
    /// intended to be used in a window (which is true for most visual widgets).
    pub fn window_or_err(&self) -> Result<&SharedWindow> {
        self.window.as_ref().context("no window")
    }

    /// Returns ID of the window associated with this widget.
    ///
    /// See [window](Self::window) for more information.
    pub fn window_id(&self) -> Option<WindowId> {
        self.window.as_ref().map(|w| w.id())
    }

    pub fn app(&self) -> &App {
        &self.app
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
    /// This method can be used to tell if the widget is hidden because [`set_visible(false)`](Self::set_visible)
    /// was called on it
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
        if self.is_window_root() {
            if let Some(window) = &self.window {
                window.set_visible(value);
            } else {
                error!("missing window for window root");
            }
        }
        self.size_hint_changed(); // trigger layout
        self
    }

    pub(crate) fn style(&self) -> Option<&str> {
        self.style.as_ref().map(|s| s.code.as_str())
    }

    pub(crate) fn set_style(&mut self, style: &str) -> &mut Self {
        if self.style() == Some(style) {
            return self;
        }
        self.style = load_css(style)
            .or_report_err()
            .map(|style_sheet| CustomStyle {
                style_sheet,
                code: style.into(),
            });
        self
    }

    pub fn compute_style<T: ComputedElementStyle>(&self) -> Rc<T> {
        self.app.style().get(
            &self.style_selector,
            self.scale(),
            self.style.as_ref().map(|s| &s.style_sheet),
        )
    }

    /// True if this widget is a root widget of an OS window.
    ///
    /// This is true for [Window](crate::Window) and false for all other provided widget types.
    pub fn is_window_root(&self) -> bool {
        self.flags.contains(Flags::window_root)
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

    fn refresh_common_style(&mut self) {
        self.base_style = self.compute_style();
        self.size_hint_changed();
        self.update();
    }

    // TODO: more straightforward API to query computed style in one call

    /// Returns a CSS-like selector for querying style properties.
    ///
    /// `StyleSelector` includes the tag name (widget type name) as well as
    /// current classes and pseudoclasses.
    pub fn style_selector(&self) -> &StyleSelector {
        &self.style_selector
    }

    pub(crate) fn style_selector_mut(&mut self) -> &mut StyleSelector {
        &mut self.style_selector
    }

    /// Scale of the widget.
    ///
    /// Scale is the multiplication factor that's used when measurements in style definitions
    /// ([crate::types::LogicalPixels]) are converted to pixel measurements used for
    /// layout, drawing, and event processing ([crate::types::PhysicalPixels]).
    ///
    /// By default, scale is determined by the scale factor reported by the OS for the OS window
    /// that contains the widget. Scale may be overriden with [AppBuilder::set_scale](crate::AppBuilder::with_scale),
    /// in which case the OS scale is ignored.
    ///
    /// Scale can be changed manually for any widget
    /// with [WidgetExt::set_scale](crate::widgets::WidgetExt::set_scale).
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
    /// [WidgetExt::set_scale](crate::widgets::WidgetExt::set_scale), if any.
    ///
    /// It returns `None` if `set_scale` hasn't been called for this widget.
    /// This value doesn't depend on the parent scale. In most cases, [scale](Self::scale) is more suitable,
    /// as that one always returns the current effective scale of the widget.
    pub fn self_scale(&self) -> Option<f32> {
        self.self_scale
    }

    /// Check if the accessibility node hasn't been disabled for this widget.
    ///
    /// This value corresponds to the value set by [set_accessibility_node_enabled](Self::set_accessibility_node_enabled).
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
    /// Returns the value that was previously set with [set_cursor_icon](Self::set_cursor_icon).
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
}

/// <h2>Actions</h2>
impl WidgetBase {
    // TODO: automate updating everything including size hint; add flags to disable it

    /// Request redraw, accessibility node update and children update of a widget.
    ///
    /// The widget will receive [handle_declare_children_request](Widget::handle_declare_children_request),
    /// [handle_accessibility_node_request](Widget::handle_accessibility_node_request) and
    /// [handle_draw](Widget::handle_draw) shortly.
    ///
    /// Note that this function is called automatically in many cases.
    pub fn update(&mut self) {
        if let Some(window) = &self.window {
            window.request_redraw();
            window.request_accessibility_update(self.address.clone());
        };
        self.app.request_children_update(self.address.clone());
    }

    pub fn ensure_visible(&self) {
        let Some(rect) = self.rect_in_self() else {
            warn!("ensure_visible: no geometry");
            return;
        };
        self.ensure_rect_visible(rect);
    }

    pub fn ensure_rect_visible(&self, rect: Rect) {
        let Some(window) = &self.window else {
            warn!("scroll_to_rect: no window");
            return;
        };
        // TODO: rename
        self.app.scroll_to_rect(window.id(), self.id, rect);
    }

    pub fn set_focus(&self, reason: FocusReason) {
        let Some(window) = &self.window else {
            warn!("set_focus: no window");
            return;
        };
        self.app.set_focus(window.id(), self.id, reason);
    }
}

/// <h2>Child widgets management</h2>
impl WidgetBase {
    // TODO: link to doc about keys
    /// Returns true if the widget contains a direct child identified by `key`.
    pub fn has_child(&self, key: impl Into<ChildKey>) -> bool {
        self.children.contains_key(&key.into())
    }
    // TODO: auto add sorting key as well, or create a key that sorts correctly.

    /// Creates a child widget of type `T` associated with `key` or returns an existing widget that has the same type and key.
    ///
    /// If there was no child with this `key`, a new child widget will be created and inserted into this widget.
    /// `set_child` will return a reference to the created widget.
    ///
    /// If there was a child with this `key` and type `T`, `set_child` returns the reference to that child.
    ///
    /// If there was a child with this `key` but with a type other than `T`, the existing child will be deleted,
    /// and a new child widget will be created and inserted into this widget.
    pub fn set_child<WI: WidgetInitializer>(
        &mut self,
        key: impl Into<ChildKey>,
        initializer: WI,
    ) -> &mut WI::Output {
        let key = key.into();
        if let Some(old_widget) = self
            .children
            .get_mut(&key)
            .and_then(|c| c.downcast_mut::<WI::Output>())
        {
            initializer.reinit(old_widget);
            // Should be `return old_widget` but borrow checker is not smart enough.
            // Should we use `polonius-the-crab` crate?
            return self
                .children
                .get_mut(&key)
                .and_then(|c| c.downcast_mut::<WI::Output>())
                .unwrap();
        }

        let new_id = RawWidgetId::new_unique();
        let ctx = if WI::Output::is_window_root_type() {
            let new_window = SharedWindow::new(new_id, &self.app);
            self.app.add_window(&new_window);
            self.new_creation_context(new_id, key.clone(), Some(new_window.clone()))
        } else {
            self.new_creation_context(new_id, key.clone(), None)
        };
        // This may delete the old widget.
        self.children.insert(
            key.clone(),
            Box::new(initializer.init(WidgetBase::new::<WI::Output>(ctx))),
        );
        self.size_hint_changed();
        self.children.get_mut(&key).unwrap().downcast_mut().unwrap()
    }

    /// Get a dyn reference to the direct child associated with `key`.
    ///
    /// Returns an error if there is no such child.
    pub fn get_dyn_child(&self, key: impl Into<ChildKey>) -> anyhow::Result<&dyn Widget> {
        Ok(self
            .children
            .get(&key.into())
            .context("no such key")?
            .as_ref())
    }

    /// Get a mutable dyn reference to the direct child associated with `key`.
    ///
    /// Returns an error if there is no such child.
    pub fn get_dyn_child_mut(
        &mut self,
        key: impl Into<ChildKey>,
    ) -> anyhow::Result<&mut dyn Widget> {
        Ok(self
            .children
            .get_mut(&key.into())
            .context("no such key")?
            .as_mut())
    }

    /// Get a reference to the direct child of type `T` associated with `key`.
    ///
    /// Returns an error if there is no such child or if the child has a type other than `T`.
    pub fn get_child<T: Widget>(&self, key: impl Into<ChildKey>) -> anyhow::Result<&T> {
        // TODO: custom error type
        self.children
            .get(&key.into())
            .context("no such key")?
            .downcast_ref()
            .context("child type mismatch")
    }

    /// Get a mutable reference to the direct child of type `T` associated with `key`.
    ///
    /// Returns an error if there is no such child or if the child has a type other than `T`.
    pub fn get_child_mut<T: Widget>(&mut self, key: impl Into<ChildKey>) -> anyhow::Result<&mut T> {
        self.children
            .get_mut(&key.into())
            .context("no such key")?
            .downcast_mut()
            .context("child type mismatch")
    }

    /// Get a reference to the direct or indirect child of type `T` identified by `id`.
    ///
    /// Returns an error if there is no such child or if the child has a type other than `T`.
    pub fn find_child<W: Widget>(&self, id: WidgetId<W>) -> Result<&W, WidgetNotFound> {
        let w = self.find_dyn_child(id.raw())?;
        Ok(w.downcast_ref::<W>().expect("child type mismatch"))
    }

    /// Get a mutable dyn reference to the direct or indirect child identified by `id`.
    ///
    /// Returns an error if there is no such child.
    pub fn find_dyn_child(&self, id: RawWidgetId) -> Result<&dyn Widget, WidgetNotFound> {
        // TODO: speed up
        for child in self.children.values() {
            if child.base().id == id {
                return Ok(child.as_ref());
            }
            if let Ok(widget) = child.base().find_dyn_child(id) {
                return Ok(widget);
            }
        }
        Err(WidgetNotFound)
    }

    /// Get a mutable reference to the direct or indirect child of type `T` identified by `id`.
    ///
    /// Returns an error if there is no such child or if the child has a type other than `T`.
    pub fn find_child_mut<W: Widget>(&mut self, id: WidgetId<W>) -> Result<&mut W, WidgetNotFound> {
        let w = self.find_dyn_child_mut(id.raw())?;
        Ok(w.downcast_mut::<W>().expect("child type mismatch"))
    }

    /// Get a mutable dyn reference to the direct or indirect child identified by `id`.
    ///
    /// Returns an error if there is no such child.
    pub fn find_dyn_child_mut(
        &mut self,
        id: RawWidgetId,
    ) -> Result<&mut dyn Widget, WidgetNotFound> {
        // TODO: speed up
        for child in self.children.values_mut() {
            if child.base().id == id {
                return Ok(child.as_mut());
            }
            if let Ok(widget) = child.base_mut().find_dyn_child_mut(id) {
                return Ok(widget);
            }
        }
        Err(WidgetNotFound)
    }

    /// Returns an iterator over the widget's children.
    pub fn children(&self) -> impl Iterator<Item = &dyn Widget> {
        self.children.values().map(|v| &**v)
    }

    /// Returns an iterator over the widget's children.
    pub fn children_mut(&mut self) -> impl Iterator<Item = &mut dyn Widget> {
        self.children.values_mut().map(|v| &mut **v)
    }

    /// Returns an iterator over the widget's children and associated keys.
    pub fn children_with_keys(&self) -> impl Iterator<Item = (&ChildKey, &dyn Widget)> {
        self.children.iter().map(|(k, v)| (k, &**v))
    }

    /// Returns an iterator over the widget's children and associated keys.
    pub fn children_with_keys_mut(&mut self) -> impl Iterator<Item = (&ChildKey, &mut dyn Widget)> {
        self.children.iter_mut().map(|(k, v)| (k, &mut **v))
    }

    /// Returns an iterator over the keys of the widget's children.
    pub fn child_keys(&self) -> impl Iterator<Item = &ChildKey> {
        self.children.keys()
    }

    /// Removes a direct child associated with `key`.
    ///
    /// Returns an error if there is no such child.
    pub fn remove_child(&mut self, key: impl Into<ChildKey>) -> Result<(), WidgetNotFound> {
        self.children.remove(&key.into()).ok_or(WidgetNotFound)?;
        self.size_hint_changed();
        Ok(())
    }

    /// Removes a direct or indirect child identified by `id`.
    ///
    /// Returns an error if there is no such child.
    pub fn remove_child_by_id(&mut self, id: RawWidgetId) -> Result<(), WidgetNotFound> {
        if id == self.id {
            warn!("remove_child_by_id: cannot delete self");
            return Err(WidgetNotFound);
        }
        let Some(address) = self.app.address(id) else {
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

    pub(crate) fn remove_children_except(&mut self, except: &HashSet<ChildKey>) {
        let keys = self
            .children
            .keys()
            .filter(|key| !except.contains(key))
            .cloned()
            .collect_vec();
        for key in keys {
            self.remove_child(key).or_report_err();
        }
    }
}

/// <h2>Size and position</h2>
impl WidgetBase {
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
}

/// <h2>Layout configuration</h2>
impl WidgetBase {
    /// Returns current layout strategy of the widget.
    ///
    /// Layout strategy determines how child widgets are positioned within the widget.
    pub fn layout(&self) -> Layout {
        self.layout
    }

    /// Set the layout strategy for the widget.
    ///
    /// Layout strategy determines how child widgets are positioned within the widget.
    pub fn set_layout(&mut self, layout: Layout) -> &mut Self {
        self.layout = layout;
        self
    }

    /// Returns current layout item configuration of this widget.
    ///
    /// This configuration influences size and position of the widget within its parent
    /// widget's layout.
    pub fn layout_item_options(&self) -> &LayoutItemOptions {
        &self.layout_item_options
    }

    /// Set layout item configuration of this widget.
    ///
    /// This will override any previously set options.
    ///
    /// This configuration influences size and position of the widget within its parent
    /// widget's layout.
    pub fn set_layout_item_options(&mut self, options: LayoutItemOptions) -> &mut Self {
        if self.layout_item_options == options {
            return self;
        }
        self.layout_item_options = options;
        self.size_hint_changed();
        self
    }

    /// Assign column `x` and row `y` to this widget in the parent widget's grid.
    ///
    /// This setting only takes effect if the parent's layout is [Layout::ExplicitGrid].
    /// In other layout modes, column and row are assigned automatically.
    pub fn set_grid_cell(&mut self, x: i32, y: i32) -> &mut Self {
        if self.layout_item_options.x().grid_cell() == Some(x..=x)
            && self.layout_item_options.y().grid_cell() == Some(y..=y)
        {
            return self;
        }
        self.layout_item_options.set_grid_cell(x, y);
        self.size_hint_changed();
        self
    }
    // TODO: setters for other layout item options (alignment, is_fixed)?

    // TODO: automate size_hint_changed?
    // TODO: add link to layout docs

    /// Request size hint re-evaluation and layout of the widget.
    ///
    /// In most cases it's not necessary to call this function.
    ///
    /// When implementing a custom layout for your widget, call this function when
    /// values of [Widget::handle_size_hint_x_request] and/or
    /// [Widget::handle_size_hint_y_request] have changed or when a layout should
    /// be performed on the widget. This may trigger a layout of other widgets.
    pub fn size_hint_changed(&mut self) {
        self.clear_size_hint_cache();
        let Some(window) = &self.window else {
            return;
        };
        window.invalidate_size_hint(self.address.clone());
    }

    pub(crate) fn clear_size_hint_cache(&mut self) {
        let mut cache = self.cache.borrow_mut();
        cache.size_hint_x.clear();
        cache.size_hint_y.clear();
    }

    pub(crate) fn size_hint_x_cache(&self, size_y: Option<PhysicalPixels>) -> Option<SizeHint> {
        self.cache.borrow().size_hint_x.get(&size_y).cloned()
    }

    pub(crate) fn set_size_hint_x_cache(&self, size_y: Option<PhysicalPixels>, value: SizeHint) {
        self.cache.borrow_mut().size_hint_x.insert(size_y, value);
    }

    pub(crate) fn size_hint_y_cache(&self, size_x: PhysicalPixels) -> Option<SizeHint> {
        self.cache.borrow().size_hint_y.get(&size_x).cloned()
    }

    pub(crate) fn set_size_hint_y_cache(&self, size_x: PhysicalPixels, value: SizeHint) {
        self.cache.borrow_mut().size_hint_y.insert(size_x, value);
    }
}

/// <h2>Shortcuts and event filters</h2>
impl WidgetBase {
    /// Add an event filter to this widget.
    ///
    /// Event filter is a function that runs for every event dispatched to the widget.
    /// It runs *before* event handlers of the widget itself.
    /// If the filter function returns `Ok(true)`, the event will be accepted (if applicable)
    /// and will not be dispatched to the widget itself or other event filters (if any).
    ///
    /// It's possible to set multiple event filters for the same widget. Event filter is identified
    /// by the `owner_id` argument. As a convention, `owner_id` should be set to
    /// the ID of the widget that calls `install_event_filter`. If `install_event_filter`
    /// is called again with the same `owner_id`, the new event filter will replace the previous one.
    ///
    /// If multiple event filters are installed, the event will be passed to each of them in
    /// an unspecified order. If any event filter returns `Ok(true)`,
    /// that event will not be dispatched to the remaining event filters.
    pub fn install_event_filter(
        &mut self,
        owner_id: RawWidgetId,
        filter: impl FnMut(Event) -> Result<bool> + 'static,
    ) {
        // TODO: deregister when owner is deleted
        self.event_filters.insert(owner_id, Box::new(filter));
    }

    /// Removes an event filter previously set with [install_event_filter](Self::install_event_filter).
    ///
    /// It removes only the event filter with the corresponding `owner_id`, leaving other
    /// event filters (if any) unchanged. If there is no matching event filter,
    /// `remove_event_filter` has no effect.
    pub fn remove_event_filter(&mut self, owner_id: RawWidgetId) {
        self.event_filters.remove(&owner_id);
    }

    // TODO: declare-compatible shortcut API
    pub fn add_shortcut(&mut self, shortcut: Shortcut) -> ShortcutId {
        let id = shortcut.id;
        if shortcut.scope == ShortcutScope::Application {
            self.app.add_shortcut(shortcut.clone());
        }
        // TODO: register widget/window shortcuts
        self.shortcuts.insert(id, shortcut);
        id
    }
    // TODO: remove_shortcut
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
        self.base.app().create_widget_callback(self.id(), func)
    }

    pub fn children(&self) -> Items<&WidgetBase> {
        Items::new(self)
    }

    pub fn children_mut(&mut self) -> ItemsMut<'_> {
        ItemsMut::new(self)
    }

    pub fn children_with_key<K: Into<ChildKey>>(&self) -> ItemsWithKey<&WidgetBase, K> {
        ItemsWithKey::new(self)
    }

    pub fn children_with_key_mut<K: Into<ChildKey>>(&mut self) -> ItemsWithKeyMut<'_, K> {
        ItemsWithKeyMut::new(self)
    }

    pub fn set_main_child<WI: WidgetInitializer>(&mut self, initializer: WI) -> &mut WI::Output {
        self.set_child(main_key(), initializer)
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

    /// Creates a callback for the widget that received this `handle_declared` call.
    pub fn callback_creator(&self) -> CallbackCreator<W> {
        CallbackCreator {
            widget_id: self.id(),
            // TODO: weak handle
            app: self.base.app.create_app_handle(),
        }
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

pub struct CallbackCreator<W> {
    pub(crate) widget_id: WidgetId<W>,
    pub(crate) app: App,
}

impl<W> CallbackCreator<W> {
    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn id(&self) -> WidgetId<W> {
        self.widget_id
    }

    /// Creates a callback for the widget that received this `handle_declared` call.
    pub fn create<E, F>(&self, func: F) -> Callback<E>
    where
        W: Widget,
        F: Fn(&mut W, E) -> anyhow::Result<()> + 'static,
        E: 'static,
    {
        self.app.create_widget_callback(self.widget_id, func)
    }
}
