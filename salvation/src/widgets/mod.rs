use {
    crate::{
        callback::{widget_callback, Callback},
        draw::DrawEvent,
        event::{
            AccessibleActionEvent, EnabledChangeEvent, Event, FocusInEvent, FocusOutEvent,
            ImeEvent, KeyboardInputEvent, LayoutEvent, MouseEnterEvent, MouseInputEvent,
            MouseLeaveEvent, MouseMoveEvent, MouseScrollEvent, ScrollToRectEvent, StyleChangeEvent,
            WindowFocusChangeEvent,
        },
        layout::{
            grid::{self, GridAxisOptions, GridOptions},
            Alignment, LayoutItemOptions, SizeHintMode, SizeHints, FALLBACK_SIZE_HINT,
        },
        shortcut::{Shortcut, ShortcutId, ShortcutScope},
        style::{
            computed::{CommonComputedStyle, ComputedElementStyle, ComputedStyle},
            css::{Element, MyPseudoClass},
            Style,
        },
        system::{address, register_address, unregister_address, with_system, ReportError},
        types::{Point, Rect, Size},
        window::{Window, WindowId},
    },
    accesskit::NodeId,
    anyhow::{Context, Result},
    derivative::Derivative,
    downcast_rs::{impl_downcast, Downcast},
    itertools::Itertools,
    log::warn,
    std::{
        collections::{btree_map, BTreeMap, HashMap},
        fmt::{self, Debug},
        marker::PhantomData,
        ops::{Deref, DerefMut},
        rc::Rc,
        sync::atomic::{AtomicU64, Ordering},
    },
    thiserror::Error,
    winit::window::{CursorIcon, WindowAttributes},
};

pub mod button;
pub mod column;
pub mod image;
pub mod label;
pub mod menu;
pub mod padding_box;
pub mod root;
pub mod row;
pub mod scroll_area;
pub mod scroll_bar;
pub mod stack;
pub mod text_input;
pub mod window;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawWidgetId(pub u64);

impl RawWidgetId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn callback<W, E, F>(self, func: F) -> Callback<E>
    where
        W: Widget,
        F: Fn(&mut W, E) -> Result<()> + 'static,
        E: 'static,
    {
        widget_callback(WidgetId::<W>::new(self), func)
    }
}

impl From<RawWidgetId> for NodeId {
    fn from(value: RawWidgetId) -> Self {
        value.0.into()
    }
}

pub struct WidgetId<T>(pub RawWidgetId, pub PhantomData<T>);

impl<T> WidgetId<T> {
    pub fn new(id: RawWidgetId) -> Self {
        Self(id, PhantomData)
    }

    pub fn callback<E, F>(self, func: F) -> Callback<E>
    where
        T: Widget,
        F: Fn(&mut T, E) -> Result<()> + 'static,
        E: 'static,
    {
        widget_callback(self, func)
    }
}

impl<T> Debug for WidgetId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<T> Clone for WidgetId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for WidgetId<T> {}

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
    // Present if the widget is mounted, not hidden, and only after layout.
    pub rect_in_window: Option<Rect>,
    // In this widget's coordinates.
    pub visible_rect: Option<Rect>,

    pub children: BTreeMap<Key, Child>,
    pub current_layout_event: Option<LayoutEvent>,

    pub size_hint_x_cache: HashMap<SizeHintMode, i32>,
    // TODO: limit count
    pub size_hint_y_cache: HashMap<(i32, SizeHintMode), i32>,
    pub size_x_fixed_cache: Option<bool>,
    pub size_y_fixed_cache: Option<bool>,

    pub is_accessible: bool,
    pub pending_accessible_update: bool,

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
}

// TODO: enum with other types
pub type Key = u64;

impl Drop for WidgetCommon {
    fn drop(&mut self) {
        unregister_address(self.id);
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
            rect_in_window: None,
            visible_rect: None,
            cursor_icon: CursorIcon::Default,
            children: BTreeMap::new(),
            size_hint_x_cache: HashMap::new(),
            size_hint_y_cache: HashMap::new(),
            size_x_fixed_cache: None,
            size_y_fixed_cache: None,
            is_accessible: true,
            pending_accessible_update: false,
            is_registered_as_focusable: false,
            event_filter: None,
            current_layout_event: None,
            is_window_root: ctx.is_window_root,
            grid_options: None,
            no_padding: false,
            shortcuts: Vec::new(),
            style_element,
            common_style,
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
                    .map(|(index, _id)| *index)
                    .unwrap_or_default() as usize,
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

    fn grid_options(&self) -> GridOptions {
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

    pub fn size(&self) -> Option<Size> {
        self.rect_in_window.as_ref().map(|g| g.size)
    }

    // Request redraw and accessible update
    pub fn update(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        window.request_redraw();
        self.pending_accessible_update = true;
    }

    pub fn has_child(&self, key: Key) -> bool {
        self.children.contains_key(&key)
    }

    // TODO: rename
    pub fn add_child_window<T: Widget>(&mut self, key: Key, _attrs: WindowAttributes) -> &mut T {
        self.add_child_internal::<T>(key, Default::default(), true)
    }

    // TODO: rename
    pub fn add_child<T: Widget>(&mut self, key: Key, options: LayoutItemOptions) -> &mut T {
        self.add_child_internal::<T>(key, options, false)
    }

    // TODO: check for row/column conflict
    // TODO: move options to child widget common
    fn add_child_internal<T: Widget>(
        &mut self,
        key: Key,
        options: LayoutItemOptions,
        is_window: bool,
    ) -> &mut T {
        let new_id = RawWidgetId::new();
        let ctx = if is_window {
            let new_window = Window::new(new_id);
            self.new_creation_context(new_id, key, Some(new_window.clone()))
        } else {
            self.new_creation_context(new_id, key, None)
        };
        match self.children.entry(key) {
            btree_map::Entry::Vacant(entry) => entry.insert(Child {
                widget: Box::new(T::new(WidgetCommon::new::<T>(ctx))),
                options,
                rect_in_parent: None,
                rect_set_during_layout: false,
            }),
            btree_map::Entry::Occupied(entry) => {
                if entry.get().widget.is::<T>() {
                    // TODO: apply layout options
                    entry.into_mut()
                } else {
                    let child = entry.into_mut();
                    // Deletes old widget.
                    *child = Child {
                        widget: Box::new(T::new(WidgetCommon::new::<T>(ctx))),
                        options,
                        rect_in_parent: None,
                        rect_set_during_layout: false,
                    };
                    child
                }
            }
        };

        self.size_hint_changed();
        self.children
            .get_mut(&key)
            .unwrap()
            .widget
            .downcast_mut()
            .unwrap()
    }

    pub fn get_child<T: Widget>(&self, key: Key) -> anyhow::Result<&T> {
        self.children
            .get(&key)
            .context("no such key")?
            .widget
            .downcast_ref()
            .context("child type mismatch")
    }

    pub fn get_child_mut<T: Widget>(&mut self, key: Key) -> anyhow::Result<&mut T> {
        self.children
            .get_mut(&key)
            .context("no such key")?
            .widget
            .downcast_mut()
            .context("child type mismatch")
    }

    pub fn set_child_options(&mut self, key: Key, options: LayoutItemOptions) -> Result<()> {
        self.children.get_mut(&key).context("no such key")?.options = options;
        self.size_hint_changed();
        Ok(())
    }

    pub fn remove_child(&mut self, key: Key) -> Result<()> {
        self.children.remove(&key).context("no such key")?;
        self.size_hint_changed();
        Ok(())
    }

    pub fn set_child_rect(&mut self, key: Key, rect_in_parent: Option<Rect>) -> Result<()> {
        let child = self
            .children
            .get_mut(&key)
            .context("set_child_rect: invalid child index")?;

        let rect_in_window = if let Some(rect_in_window) = self.rect_in_window {
            rect_in_parent.map(|rect_in_parent| rect_in_parent.translate(rect_in_window.top_left))
        } else {
            None
        };
        let visible_rect = if let (Some(visible_rect), Some(rect_in_parent)) =
            (self.visible_rect, rect_in_parent)
        {
            Some(
                visible_rect
                    .translate(-rect_in_parent.top_left)
                    .intersect(Rect::from_pos_size(Point::default(), rect_in_parent.size)),
            )
            .filter(|r| r != &Rect::default())
        } else {
            None
        };
        child.rect_in_parent = rect_in_parent;
        // println!(
        //     "rect_in_window {:?} -> {:?}",
        //     child.widget.common().rect_in_window,
        //     rect_in_window
        // );
        // println!(
        //     "visible_rect {:?} -> {:?}",
        //     child.widget.common().visible_rect,
        //     visible_rect
        // );
        let rects_changed = child.widget.common().rect_in_window != rect_in_window
            || child.widget.common().visible_rect != visible_rect;
        if let Some(event) = &self.current_layout_event {
            if rects_changed || event.size_hints_changed_within(child.widget.common().address()) {
                //println!("set_child_rect ok1");
                child.widget.dispatch(
                    LayoutEvent {
                        new_rect_in_window: rect_in_window,
                        new_visible_rect: visible_rect,
                        changed_size_hints: event.changed_size_hints.clone(),
                    }
                    .into(),
                );
            }
            child.rect_set_during_layout = true;
        } else {
            if rects_changed {
                //println!("set_child_rect ok2");
                child.widget.dispatch(
                    LayoutEvent {
                        new_rect_in_window: rect_in_window,
                        new_visible_rect: visible_rect,
                        changed_size_hints: Vec::new(),
                    }
                    .into(),
                );
            }
        }
        //println!("set_child_rect end");
        Ok(())
    }

    pub fn set_child_rects(&mut self, rects: &BTreeMap<Key, Rect>) -> Result<()> {
        for (key, rect) in rects {
            self.set_child_rect(*key, Some(*rect))?;
        }
        Ok(())
    }

    pub fn size_hint_changed(&mut self) {
        self.clear_size_hint_cache();
        let Some(window) = &self.window else {
            return;
        };
        window.invalidate_size_hint(self.address.clone());
    }

    fn clear_size_hint_cache(&mut self) {
        self.size_hint_x_cache.clear();
        self.size_hint_y_cache.clear();
        self.size_x_fixed_cache = None;
        self.size_y_fixed_cache = None;
    }

    pub fn window_or_err(&self) -> Result<&Window> {
        self.window.as_ref().context("no window")
    }

    pub fn address(&self) -> &WidgetAddress {
        &self.address
    }

    pub fn rect_in_window_or_err(&self) -> Result<Rect> {
        self.rect_in_window.context("no rect_in_window")
    }

    pub fn size_or_err(&self) -> Result<Size> {
        Ok(self.rect_in_window.context("no rect_in_window")?.size)
    }

    pub fn rect_or_err(&self) -> Result<Rect> {
        Ok(Rect::from_pos_size(
            Point::default(),
            self.rect_in_window.context("no rect_in_window")?.size,
        ))
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

    fn enabled_changed(&mut self) {
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

    fn focused_changed(&mut self) {
        if self.is_focused() {
            self.style_element.add_pseudo_class(MyPseudoClass::Focus);
        } else {
            self.style_element.remove_pseudo_class(MyPseudoClass::Focus);
        }
        self.refresh_common_style();
    }

    fn mouse_over_changed(&mut self) {
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
        //     child.widget.common_mut().unmount_accessible();
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
            if child.widget.common().id == id {
                return Ok(child.widget.as_mut());
            }
            if let Ok(widget) = child.widget.common_mut().widget_raw(id) {
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

    fn refresh_common_style(&mut self) {
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
}

#[derive(Debug)]
pub struct WidgetCommonTyped<T> {
    common: WidgetCommon,
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

    pub fn add_child_window<T: Widget>(&mut self, key: Key, attrs: WindowAttributes) -> &mut T {
        self.common.add_child_window::<T>(key, attrs)
    }

    pub fn add_child<T: Widget>(&mut self, key: Key, options: LayoutItemOptions) -> &mut T {
        self.common.add_child::<T>(key, options)
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

#[derive(Debug, Error)]
#[error("widget not found")]
pub struct WidgetNotFound;

pub fn get_widget_by_address_mut<'a>(
    root_widget: &'a mut dyn Widget,
    address: &WidgetAddress,
) -> Result<&'a mut dyn Widget, WidgetNotFound> {
    let root_address = &root_widget.common().address;

    if !address.starts_with(root_address) {
        warn!("get_widget_by_address_mut: address is not within root widget");
        return Err(WidgetNotFound);
    }
    let root_address_len = root_address.path.len();
    let mut current_widget = root_widget;
    for (key, _id) in &address.path[root_address_len..] {
        current_widget = current_widget
            .common_mut()
            .children
            .get_mut(key)
            .ok_or(WidgetNotFound)?
            .widget
            .as_mut();
    }
    Ok(current_widget)
}

pub fn get_widget_by_id_mut(
    root_widget: &mut dyn Widget,
    id: RawWidgetId,
) -> Result<&mut dyn Widget, WidgetNotFound> {
    let address = address(id).ok_or(WidgetNotFound)?;
    get_widget_by_address_mut(root_widget, &address)
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Child {
    #[derivative(Debug = "ignore")]
    pub widget: Box<dyn Widget>,
    pub options: LayoutItemOptions,
    pub rect_in_parent: Option<Rect>,
    pub rect_set_during_layout: bool,
}

pub trait Widget: Downcast {
    fn type_name() -> &'static str
    where
        Self: Sized;

    fn is_window_root_type() -> bool
    where
        Self: Sized,
    {
        false
    }

    fn new(common: WidgetCommonTyped<Self>) -> Self
    where
        Self: Sized;
    fn common(&self) -> &WidgetCommon;
    fn common_mut(&mut self) -> &mut WidgetCommon;
    fn handle_draw(&mut self, event: DrawEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_mouse_input(&mut self, event: MouseInputEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_mouse_scroll(&mut self, event: MouseScrollEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_mouse_enter(&mut self, event: MouseEnterEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_mouse_move(&mut self, event: MouseMoveEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_mouse_leave(&mut self, event: MouseLeaveEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_keyboard_input(&mut self, event: KeyboardInputEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_ime(&mut self, event: ImeEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        let options = self.common().grid_options();
        let Some(size) = self.common().size() else {
            return Ok(());
        };
        let rects = grid::layout(&mut self.common_mut().children, &options, size)?;
        self.common_mut().set_child_rects(&rects)
    }
    fn handle_scroll_to_rect(&mut self, event: ScrollToRectEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_focus_in(&mut self, event: FocusInEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_focus_out(&mut self, event: FocusOutEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_window_focus_change(&mut self, event: WindowFocusChangeEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_accessible_action(&mut self, event: AccessibleActionEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_style_change(&mut self, event: StyleChangeEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_enabled_change(&mut self, event: EnabledChangeEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_event(&mut self, event: Event) -> Result<bool> {
        match event {
            Event::MouseInput(e) => self.handle_mouse_input(e),
            Event::MouseScroll(e) => self.handle_mouse_scroll(e),
            Event::MouseEnter(e) => self.handle_mouse_enter(e),
            Event::MouseMove(e) => self.handle_mouse_move(e),
            Event::MouseLeave(e) => self.handle_mouse_leave(e).map(|()| true),
            Event::KeyboardInput(e) => self.handle_keyboard_input(e),
            Event::Ime(e) => self.handle_ime(e),
            Event::Draw(e) => self.handle_draw(e).map(|()| true),
            Event::Layout(e) => self.handle_layout(e).map(|()| true),
            Event::FocusIn(e) => self.handle_focus_in(e).map(|()| true),
            Event::FocusOut(e) => self.handle_focus_out(e).map(|()| true),
            Event::WindowFocusChange(e) => self.handle_window_focus_change(e).map(|()| true),
            Event::Accessible(e) => self.handle_accessible_action(e).map(|()| true),
            Event::ScrollToRect(e) => self.handle_scroll_to_rect(e),
            Event::StyleChange(e) => self.handle_style_change(e).map(|()| true),
            Event::EnabledChange(e) => self.handle_enabled_change(e).map(|()| true),
        }
    }
    fn recalculate_size_hint_x(&mut self, mode: SizeHintMode) -> Result<i32> {
        let options = self.common().grid_options();
        grid::size_hint_x(&mut self.common_mut().children, &options, mode)
    }
    fn recalculate_size_hint_y(&mut self, size_x: i32, mode: SizeHintMode) -> Result<i32> {
        let options = self.common().grid_options();
        grid::size_hint_y(&mut self.common_mut().children, &options, size_x, mode)
    }
    fn recalculate_size_x_fixed(&mut self) -> bool {
        let options = self.common().grid_options();
        grid::size_x_fixed(&mut self.common_mut().children, &options)
    }
    fn recalculate_size_y_fixed(&mut self) -> bool {
        let options = self.common().grid_options();
        grid::size_y_fixed(&mut self.common_mut().children, &options)
    }

    // TODO: result?
    fn accessible_node(&mut self) -> Option<accesskit::NodeBuilder> {
        None
    }
}
impl_downcast!(Widget);

pub struct WidgetWithId<W> {
    pub id: WidgetId<W>,
    pub widget: W,
}

pub trait WidgetExt {
    fn id(&self) -> WidgetId<Self>
    where
        Self: Sized;

    fn callback<F, E>(&self, func: F) -> Callback<E>
    where
        F: Fn(&mut Self, E) -> Result<()> + 'static,
        E: 'static,
        Self: Sized;

    fn with_id(self) -> WidgetWithId<Self>
    where
        Self: Sized,
    {
        WidgetWithId {
            id: self.id(),
            widget: self,
        }
    }

    fn set_no_padding(&mut self, no_padding: bool) -> &mut Self;
    fn set_visible(&mut self, value: bool) -> &mut Self;
    fn set_focusable(&mut self, value: bool) -> &mut Self;
    fn set_accessible(&mut self, value: bool) -> &mut Self;
    fn add_pseudo_class(&mut self, class: MyPseudoClass) -> &mut Self;

    fn dispatch(&mut self, event: Event) -> bool;
    fn update_accessible(&mut self);
    fn size_hint_x(&mut self, mode: SizeHintMode) -> i32;
    fn size_hints_x(&mut self) -> SizeHints;
    fn size_hint_y(&mut self, size_x: i32, mode: SizeHintMode) -> i32;
    fn size_hints_y(&mut self, size_x: i32) -> SizeHints;
    fn size_hints_y_from_hints_x(&mut self, hints_x: SizeHints) -> SizeHints;
    fn size_x_fixed(&mut self) -> bool;
    fn size_y_fixed(&mut self) -> bool;

    // TODO: private
    fn set_enabled(&mut self, enabled: bool);
    fn set_style(&mut self, style: Option<Rc<Style>>) -> Result<()>;
    fn add_class(&mut self, class: &'static str) -> &mut Self;
    fn remove_class(&mut self, class: &'static str);

    fn boxed(self) -> Box<dyn Widget>
    where
        Self: Sized;
}

impl<W: Widget + ?Sized> WidgetExt for W {
    fn id(&self) -> WidgetId<Self>
    where
        Self: Sized,
    {
        WidgetId(self.common().id, PhantomData)
    }

    // TODO: use classes instead?
    fn set_no_padding(&mut self, no_padding: bool) -> &mut Self {
        self.common_mut().set_no_padding(no_padding);
        self
    }

    fn set_visible(&mut self, value: bool) -> &mut Self {
        if self.common_mut().is_self_visible == value {
            return self;
        }
        self.common_mut().is_self_visible = value;
        self.common_mut().size_hint_changed(); // trigger layout
        self
    }

    fn set_focusable(&mut self, value: bool) -> &mut Self {
        self.common_mut().set_focusable(value);
        self
    }
    fn set_accessible(&mut self, value: bool) -> &mut Self {
        self.common_mut().set_accessible(value);
        self
    }

    fn add_pseudo_class(&mut self, class: MyPseudoClass) -> &mut Self {
        self.common_mut().add_pseudo_class(class);
        self
    }

    fn callback<F, E>(&self, func: F) -> Callback<E>
    where
        F: Fn(&mut Self, E) -> Result<()> + 'static,
        E: 'static,
        Self: Sized,
    {
        widget_callback(self.id(), func)
    }

    fn dispatch(&mut self, event: Event) -> bool {
        let mut accepted = false;
        let mut should_dispatch = true;
        match &event {
            Event::FocusIn(_) => {
                self.common_mut().is_focused = true;
                self.common_mut().focused_changed();
            }
            Event::FocusOut(_) => {
                self.common_mut().is_focused = false;
                self.common_mut().focused_changed();
            }
            Event::WindowFocusChange(e) => {
                self.common_mut().is_window_focused = e.is_focused;
                self.common_mut().focused_changed();
            }
            Event::MouseInput(event) => {
                should_dispatch = self.common().is_enabled();
                if should_dispatch {
                    for child in self.common_mut().children.values_mut().rev() {
                        if let Some(rect_in_parent) = child.rect_in_parent {
                            if let Some(child_event) = event.map_to_child(
                                rect_in_parent,
                                child.widget.common().receives_all_mouse_events,
                            ) {
                                if child.widget.dispatch(child_event.into()) {
                                    accepted = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Event::MouseScroll(event) => {
                should_dispatch = self.common().is_enabled();
                if should_dispatch {
                    for child in self.common_mut().children.values_mut().rev() {
                        if let Some(rect_in_parent) = child.rect_in_parent {
                            if let Some(child_event) = event.map_to_child(
                                rect_in_parent,
                                child.widget.common().receives_all_mouse_events,
                            ) {
                                if child.widget.dispatch(child_event.into()) {
                                    accepted = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Event::MouseEnter(_) | Event::KeyboardInput(_) | Event::Ime(_) => {
                should_dispatch = self.common().is_enabled();
            }
            Event::MouseMove(event) => {
                should_dispatch = self.common().is_enabled();
                if should_dispatch {
                    for child in self.common_mut().children.values_mut().rev() {
                        if let Some(rect_in_parent) = child.rect_in_parent {
                            if let Some(child_event) = event.map_to_child(
                                rect_in_parent,
                                child.widget.common().receives_all_mouse_events,
                            ) {
                                if child.widget.dispatch(child_event.into()) {
                                    accepted = true;
                                    break;
                                }
                            }
                        }
                    }

                    if !accepted {
                        let is_enter =
                            if let Some(window) = self.common().window_or_err().or_report_err() {
                                !window.is_mouse_entered(self.common().id)
                            } else {
                                false
                            };

                        if is_enter {
                            self.dispatch(event.create_enter_event().into());
                        }
                    }
                }
            }
            Event::MouseLeave(_) => {
                self.common_mut().is_mouse_over = false;
                self.common_mut().mouse_over_changed();
                should_dispatch = self.common().is_enabled();
            }
            Event::Layout(event) => {
                self.common_mut().rect_in_window = event.new_rect_in_window;
                self.common_mut().visible_rect = event.new_visible_rect;
                self.common_mut().current_layout_event = Some(event.clone());
                for child in self.common_mut().children.values_mut() {
                    child.rect_set_during_layout = false;
                }
            }
            Event::StyleChange(_) => {
                self.common_mut().refresh_common_style();
            }
            Event::EnabledChange(_) => {
                self.common_mut().enabled_changed();
            }
            _ => {}
        }
        if !accepted && should_dispatch {
            if let Some(event_filter) = &mut self.common_mut().event_filter {
                accepted = event_filter(event.clone()).or_report_err().unwrap_or(false);
            }
            if !accepted {
                accepted = self
                    .handle_event(event.clone())
                    .or_report_err()
                    .unwrap_or(false);
            }
        }
        match event {
            Event::MouseInput(_) | Event::MouseScroll(_) => {
                if accepted {
                    let common = self.common_mut();
                    if let Some(window) = common.window_or_err().or_report_err() {
                        if window
                            .current_mouse_event_state()
                            .or_report_err()
                            .map_or(false, |e| !e.is_accepted())
                        {
                            window.accept_current_mouse_event(common.id).or_report_err();
                        }
                    }
                }
            }
            Event::MouseEnter(_) => {
                accept_mouse_move_or_enter_event(self, true);
            }
            Event::MouseMove(_) => {
                accept_mouse_move_or_enter_event(self, false);
            }
            Event::Draw(event) => {
                for child in self.common_mut().children.values_mut() {
                    if let Some(rect_in_parent) = child.rect_in_parent {
                        if let Some(child_event) = event.map_to_child(rect_in_parent) {
                            child.widget.dispatch(child_event.into());
                        }
                    }
                }
            }
            Event::WindowFocusChange(event) => {
                for child in self.common_mut().children.values_mut() {
                    child.widget.dispatch(event.clone().into());
                }
            }
            Event::FocusIn(_) | Event::FocusOut(_) | Event::MouseLeave(_) => {
                self.common_mut().update();
            }
            Event::Layout(_) => {
                // TODO: optimize
                let keys = self.common().children.keys().copied().collect_vec();
                for key in keys {
                    if !self
                        .common()
                        .children
                        .get(&key)
                        .unwrap()
                        .rect_set_during_layout
                    {
                        let rect_in_parent =
                            self.common().children.get(&key).unwrap().rect_in_parent;
                        self.common_mut()
                            .set_child_rect(key, rect_in_parent)
                            .or_report_err();
                    }
                    self.common_mut()
                        .children
                        .get_mut(&key)
                        .unwrap()
                        .rect_set_during_layout = false;
                }
                self.common_mut().current_layout_event = None;
                self.common_mut().update();
            }
            Event::ScrollToRect(event) => {
                if !accepted && event.address != self.common().address {
                    if event.address.starts_with(&self.common().address) {
                        if let Some((key, id)) = event.address.item_at(self.common().address.len())
                        {
                            if let Some(child) = self.common_mut().children.get_mut(&key) {
                                if child.widget.common().id == id {
                                    child.widget.dispatch(event.clone().into());
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
            }
            Event::EnabledChange(event) => {
                for child in self.common_mut().children.values_mut() {
                    let old_enabled = child.widget.common_mut().is_enabled();
                    child.widget.common_mut().is_parent_enabled = event.is_enabled;
                    let new_enabled = child.widget.common_mut().is_enabled();
                    if old_enabled != new_enabled {
                        child.widget.dispatch(
                            EnabledChangeEvent {
                                is_enabled: new_enabled,
                            }
                            .into(),
                        );
                        // TODO: do it when pseudo class changes instead
                        child.widget.dispatch(StyleChangeEvent {}.into());
                    }
                }
            }
            Event::StyleChange(event) => {
                for child in self.common_mut().children.values_mut() {
                    // TODO: only if really changed
                    child.widget.dispatch(event.clone().into());
                }
            }
            Event::KeyboardInput(_) | Event::Ime(_) | Event::Accessible(_) => {}
        }

        self.update_accessible();
        accepted
    }

    fn update_accessible(&mut self) {
        if !self.common().pending_accessible_update {
            return;
        }
        let node = if self.common().is_accessible {
            self.accessible_node()
        } else {
            None
        };

        let Some(window) = self.common().window.as_ref() else {
            return;
        };
        let rect = self.common().rect_in_window;
        let node = node.map(|mut node| {
            if let Some(rect) = rect {
                node.set_bounds(accesskit::Rect {
                    x0: rect.top_left.x as f64,
                    y0: rect.top_left.y as f64,
                    x1: rect.bottom_right().x as f64,
                    y1: rect.bottom_right().y as f64,
                });
            }
            node
        });
        window.accessible_update(self.common().id.0.into(), node);
        self.common_mut().pending_accessible_update = false;
    }

    fn size_hint_x(&mut self, mode: SizeHintMode) -> i32 {
        if let Some(cached) = self.common().size_hint_x_cache.get(&mode) {
            *cached
        } else {
            let r = self
                .recalculate_size_hint_x(mode)
                .or_report_err()
                .unwrap_or(FALLBACK_SIZE_HINT);
            self.common_mut().size_hint_x_cache.insert(mode, r);
            r
        }
    }
    fn size_hints_x(&mut self) -> SizeHints {
        SizeHints {
            min: self.size_hint_x(SizeHintMode::Min),
            preferred: self.size_hint_x(SizeHintMode::Preferred),
            is_fixed: self.size_x_fixed(),
        }
    }
    fn size_hint_y(&mut self, size_x: i32, mode: SizeHintMode) -> i32 {
        if let Some(cached) = self.common().size_hint_y_cache.get(&(size_x, mode)) {
            *cached
        } else {
            let r = self
                .recalculate_size_hint_y(size_x, mode)
                .or_report_err()
                .unwrap_or(FALLBACK_SIZE_HINT);
            self.common_mut()
                .size_hint_y_cache
                .insert((size_x, mode), r);
            r
        }
    }

    fn size_hints_y(&mut self, size_x: i32) -> SizeHints {
        SizeHints {
            min: self.size_hint_y(size_x, SizeHintMode::Min),
            preferred: self.size_hint_y(size_x, SizeHintMode::Preferred),
            is_fixed: self.size_y_fixed(),
        }
    }

    fn size_hints_y_from_hints_x(&mut self, hints_x: SizeHints) -> SizeHints {
        SizeHints {
            min: self.size_hint_y(hints_x.min, SizeHintMode::Min),
            preferred: self.size_hint_y(hints_x.preferred, SizeHintMode::Preferred),
            is_fixed: self.size_y_fixed(),
        }
    }

    fn size_x_fixed(&mut self) -> bool {
        if let Some(cached) = self.common().size_x_fixed_cache {
            cached
        } else {
            let r = self.recalculate_size_x_fixed();
            self.common_mut().size_x_fixed_cache = Some(r);
            r
        }
    }
    fn size_y_fixed(&mut self) -> bool {
        if let Some(cached) = self.common().size_y_fixed_cache {
            cached
        } else {
            let r = self.recalculate_size_y_fixed();
            self.common_mut().size_y_fixed_cache = Some(r);
            r
        }
    }

    fn set_style(&mut self, style: Option<Rc<Style>>) -> Result<()> {
        let scale = self.common().parent_style.0.scale;
        let style = if let Some(style) = style {
            Some(ComputedStyle::new(style, scale)?)
        } else {
            None
        };
        self.common_mut().self_style = style;
        self.dispatch(StyleChangeEvent {}.into());
        Ok(())
    }

    fn add_class(&mut self, class: &'static str) -> &mut Self {
        self.common_mut().style_element.add_class(class);
        self.dispatch(StyleChangeEvent {}.into());
        self
    }

    fn remove_class(&mut self, class: &'static str) {
        self.common_mut().style_element.remove_class(class);
        self.dispatch(StyleChangeEvent {}.into());
    }

    fn set_enabled(&mut self, enabled: bool) {
        let old_enabled = self.common().is_enabled();
        if self.common().is_self_enabled == enabled {
            return;
        }
        self.common_mut().is_self_enabled = enabled;
        let new_enabled = self.common().is_enabled();
        if old_enabled != new_enabled {
            self.dispatch(
                EnabledChangeEvent {
                    is_enabled: new_enabled,
                }
                .into(),
            );
            // TODO: do it when pseudo class changes instead
            self.dispatch(StyleChangeEvent {}.into());
        }
    }

    fn boxed(self) -> Box<dyn Widget>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

fn accept_mouse_move_or_enter_event(widget: &mut (impl Widget + ?Sized), is_enter: bool) {
    let Some(window) = widget.common_mut().window_or_err().or_report_err() else {
        return;
    };
    if window
        .current_mouse_event_state()
        .or_report_err()
        .map_or(false, |e| !e.is_accepted())
    {
        let Some(rect_in_window) = widget.common().rect_in_window_or_err().or_report_err() else {
            return;
        };
        let Some(window) = widget.common().window_or_err().or_report_err() else {
            return;
        };
        let id = widget.common().id;
        window.accept_current_mouse_event(id).or_report_err();

        window.set_cursor(widget.common().cursor_icon);
        if is_enter {
            window.add_mouse_entered(rect_in_window, id);
            widget.common_mut().is_mouse_over = true;
            widget.common_mut().mouse_over_changed();
        }
    }
}

pub fn invalidate_size_hint_cache(widget: &mut dyn Widget, pending: &[WidgetAddress]) {
    let common = widget.common_mut();
    for pending_addr in pending {
        if pending_addr.starts_with(&common.address) {
            common.clear_size_hint_cache();
            for child in common.children.values_mut() {
                invalidate_size_hint_cache(child.widget.as_mut(), pending);
            }
            return;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WidgetAddress {
    pub path: Vec<(Key, RawWidgetId)>,
}

impl WidgetAddress {
    pub fn root(id: RawWidgetId) -> Self {
        Self {
            path: vec![(0, id)],
        }
    }
    pub fn join(mut self, key: Key, id: RawWidgetId) -> Self {
        self.path.push((key, id));
        self
    }
    pub fn starts_with(&self, base: &WidgetAddress) -> bool {
        base.path.len() <= self.path.len() && base.path == self.path[..base.path.len()]
    }
    pub fn widget_id(&self) -> RawWidgetId {
        self.path.last().expect("WidgetAddress path is empty").1
    }
    pub fn parent_widget_id(&self) -> Option<RawWidgetId> {
        if self.path.len() > 1 {
            Some(self.path[self.path.len() - 2].1)
        } else {
            None
        }
    }
    pub fn strip_prefix(&self, parent: RawWidgetId) -> Option<&[(Key, RawWidgetId)]> {
        if let Some(index) = self.path.iter().position(|(_index, id)| *id == parent) {
            Some(&self.path[index + 1..])
        } else {
            None
        }
    }
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.path.len()
    }
    pub fn item_at(&self, pos: usize) -> Option<(Key, RawWidgetId)> {
        self.path.get(pos).copied()
    }
}

#[macro_export]
macro_rules! impl_widget_common {
    () => {
        fn type_name() -> &'static str {
            std::any::type_name::<Self>().rsplit("::").next().unwrap()
        }

        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }
    };
}
