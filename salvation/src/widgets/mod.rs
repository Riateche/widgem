use {
    crate::{
        callback::{widget_callback, Callback},
        create_window,
        draw::DrawEvent,
        event::{
            AccessibleActionEvent, EnabledChangeEvent, Event, FocusInEvent, FocusOutEvent,
            ImeEvent, KeyboardInputEvent, LayoutEvent, MouseEnterEvent, MouseInputEvent,
            MouseLeaveEvent, MouseMoveEvent, MouseScrollEvent, ScrollToRectEvent, StyleChangeEvent,
            WidgetScopeChangeEvent, WindowFocusChangeEvent,
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
        window::Window,
    },
    accesskit::NodeId,
    anyhow::{bail, Context, Result},
    derivative::Derivative,
    downcast_rs::{impl_downcast, Downcast},
    log::warn,
    std::{
        collections::{BTreeMap, HashMap},
        fmt::{self, Debug},
        marker::PhantomData,
        ops::{Deref, DerefMut},
        rc::Rc,
        sync::atomic::{AtomicU64, Ordering},
    },
    thiserror::Error,
    winit::window::{CursorIcon, WindowAttributes, WindowId},
};

pub mod button;
pub mod column;
pub mod image;
pub mod label;
pub mod menu;
pub mod padding_box;
pub mod row;
pub mod scroll_area;
pub mod scroll_bar;
pub mod stack;
pub mod text_input;

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
pub struct WidgetScope {
    pub parent_id: Option<RawWidgetId>,
    pub address: WidgetAddress,
    pub window: Option<Window>,
}

impl WidgetScope {
    pub fn window_id(&self) -> Option<WindowId> {
        self.window.as_ref().map(|w| w.id())
    }
}

impl WidgetScope {
    fn new(id: RawWidgetId) -> Self {
        Self {
            parent_id: None,
            address: WidgetAddress::root(id),
            window: None,
        }
    }
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
    // TODO: set initial value in mount event
    pub is_window_focused: bool,
    pub scope: WidgetScope,

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

    pub children: Vec<Child>,
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
    pub accessible_mounted: bool,
    pub grid_options: Option<GridOptions>,
    pub no_padding: bool,

    pub shortcuts: Vec<Shortcut>,
    pub style_element: Element,
    pub common_style: Rc<CommonComputedStyle>,
}

impl Drop for WidgetCommon {
    fn drop(&mut self) {
        unregister_address(self.id);
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
    pub fn new<T: Widget>() -> WidgetCommonTyped<T> {
        let id = RawWidgetId::new();
        let scope = WidgetScope::new(id);
        register_address(id, scope.address.clone());

        let common = Self {
            id,
            receives_all_mouse_events: false,
            parent_style: with_system(|s| s.default_style.clone()),
            self_style: None,
            is_focusable: false,
            is_focused: false,
            is_parent_enabled: true,
            is_self_enabled: true,
            is_self_visible: true,
            is_mouse_over: false,
            is_window_focused: false,
            enable_ime: false,
            rect_in_window: None,
            visible_rect: None,
            cursor_icon: CursorIcon::Default,
            children: Vec::new(),
            size_hint_x_cache: HashMap::new(),
            size_hint_y_cache: HashMap::new(),
            size_x_fixed_cache: None,
            size_y_fixed_cache: None,
            is_accessible: true,
            pending_accessible_update: false,
            scope,
            is_registered_as_focusable: false,
            event_filter: None,
            current_layout_event: None,
            is_window_root: false,
            accessible_mounted: false,
            grid_options: None,
            no_padding: false,
            shortcuts: Vec::new(),
            style_element: Element::new(T::type_name()),
            // TODO: avoid allocation
            common_style: Rc::new(CommonComputedStyle::default()),
        };
        WidgetCommonTyped {
            common,
            _marker: PhantomData,
        }
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

    pub fn scope_for_child(&mut self, child_index: usize) -> WidgetScope {
        let child = &mut self.children[child_index];
        let window = if child.widget.common().is_window_root {
            child.widget.common().scope.window.clone()
        } else {
            self.scope.window.clone()
        };
        WidgetScope {
            parent_id: Some(self.id),
            address: self
                .scope
                .address
                .clone()
                .join(child_index, child.widget.common().id),
            window,
        }
    }

    pub fn size(&self) -> Option<Size> {
        self.rect_in_window.as_ref().map(|g| g.size)
    }

    // Request redraw and accessible update
    pub fn update(&mut self) {
        let Some(window) = &self.scope.window else {
            return;
        };
        window.request_redraw();
        self.pending_accessible_update = true;
    }

    // TODO: check for row/column conflict
    pub fn add_child(&mut self, widget: Box<dyn Widget>, options: LayoutItemOptions) -> usize {
        let index = self.children.len();
        self.insert_child(index, widget, options)
            .expect("should never fail with correct index");
        index
    }

    // TODO: fn replace_child
    pub fn insert_child(
        &mut self,
        index: usize,
        widget: Box<dyn Widget>,
        options: LayoutItemOptions,
    ) -> Result<()> {
        if index > self.children.len() {
            bail!("index out of bounds");
        }
        self.children.insert(
            index,
            Child {
                widget,
                options,
                rect_in_parent: None,
                rect_set_during_layout: false,
            },
        );
        self.update_children_scope(index);
        self.size_hint_changed();
        Ok(())
    }

    pub fn set_child_options(&mut self, index: usize, options: LayoutItemOptions) -> Result<()> {
        if index >= self.children.len() {
            bail!("index out of bounds");
        }
        self.children[index].options = options;
        self.size_hint_changed();
        Ok(())
    }

    fn update_children_scope(&mut self, from_index: usize) {
        let num_children = self.children.len();
        for i in from_index..num_children {
            let scope = self.scope_for_child(i);
            self.children[i].widget.as_mut().set_scope(scope);
        }
    }

    pub fn remove_child(&mut self, index: usize) -> Result<Box<dyn Widget>> {
        if index >= self.children.len() {
            bail!("invalid child index");
        }
        let mut widget = self.children.remove(index).widget;
        let id = widget.common().id;
        let window_id = widget.common().scope.window_id();
        widget.set_scope(WidgetScope::new(id));
        if widget.common().is_window_root {
            if let Some(window_id) = window_id {
                with_system(|system| {
                    system.windows.remove(&window_id);
                });
            }
        }
        self.update_children_scope(index);
        self.size_hint_changed();
        Ok(widget)
    }

    pub fn set_child_rect(&mut self, index: usize, rect_in_parent: Option<Rect>) -> Result<()> {
        let child = self
            .children
            .get_mut(index)
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

    pub fn set_child_rects(&mut self, rects: &BTreeMap<usize, Rect>) -> Result<()> {
        let len = self.children.len();
        for index in 0..len {
            self.set_child_rect(index, rects.get(&index).copied())?;
        }
        Ok(())
    }

    pub fn size_hint_changed(&mut self) {
        self.clear_size_hint_cache();
        let Some(window) = &self.scope.window else {
            return;
        };
        window.invalidate_size_hint(self.scope.address.clone());
    }

    fn clear_size_hint_cache(&mut self) {
        self.size_hint_x_cache.clear();
        self.size_hint_y_cache.clear();
        self.size_x_fixed_cache = None;
        self.size_y_fixed_cache = None;
    }

    pub fn window_or_err(&self) -> Result<&Window> {
        self.scope.window.as_ref().context("no window")
    }

    pub fn address(&self) -> &WidgetAddress {
        &self.scope.address
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
            if let Some(window) = &self.scope.window {
                if is_focusable {
                    window.add_focusable_widget(self.scope.address.clone(), self.id);
                } else {
                    window.remove_focusable_widget(self.scope.address.clone(), self.id);
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
        if !self.accessible_mounted {
            return;
        }
        // println!("unmount_accessible {:?}", self.id);
        for child in &mut self.children {
            child.widget.common_mut().unmount_accessible();
        }
        if let Some(window) = &self.scope.window {
            let root_widget_id = window.root_widget_id();
            window.accessible_unmount(
                if self.id == root_widget_id {
                    None
                } else {
                    self.scope.parent_id.map(|id| id.into())
                },
                self.id.into(),
            );
        }
        self.accessible_mounted = false;
    }

    pub fn set_scope(&mut self, scope: WidgetScope) {
        // TODO: (de)register widget/window shortcuts

        let addr_changed = self.scope.address != scope.address;
        let parent_id_changed = self.scope.parent_id != scope.parent_id;
        let window_changed = self.scope.window_id() != scope.window_id();
        let update_accessible = addr_changed || parent_id_changed || window_changed;

        if window_changed {
            self.is_focused = false;
            self.is_window_focused = false;
        }
        if window_changed && self.is_registered_as_focusable {
            if let Some(window) = &self.scope.window {
                window.remove_focusable_widget(self.scope.address.clone(), self.id);
                self.is_registered_as_focusable = false;
            }
        }
        if update_accessible {
            self.unmount_accessible();
            if let Some(window) = &self.scope.window {
                if self.scope.window.is_none() {
                    window.accessible_update(self.id.0.into(), None);
                }
            }
        }

        self.scope = scope;

        if addr_changed {
            register_address(self.id, self.scope.address.clone());
        }
        if update_accessible {
            if let Some(window) = &self.scope.window {
                let root_widget_id = window.root_widget_id();
                window.accessible_mount(
                    if self.id == root_widget_id {
                        None
                    } else if let Some(parent_id) = self.scope.parent_id {
                        Some(parent_id.into())
                    } else {
                        warn!("widget is not a window root so it must have a parent");
                        None
                    },
                    self.id.into(),
                    self.scope
                        .address
                        .path
                        .last()
                        .map(|(index, _id)| *index)
                        .unwrap_or_default(),
                );
                self.accessible_mounted = true;
            }
        }
        if let Some(window) = &self.scope.window {
            self.is_window_focused = window.is_focused();
        }
        self.update();
        self.enabled_changed();
        self.focused_changed();
        self.mouse_over_changed();
        self.register_focusable();
        self.refresh_common_style();
    }

    pub fn widget<W: Widget>(&mut self, id: WidgetId<W>) -> Result<&mut W, WidgetNotFound> {
        let w = self.widget_raw(id.0)?;
        Ok(w.downcast_mut::<W>().expect("widget downcast failed"))
    }

    pub fn widget_raw(&mut self, id: RawWidgetId) -> Result<&mut dyn Widget, WidgetNotFound> {
        // TODO: speed up
        for child in &mut self.children {
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
    pub fn callback<E, F>(&self, func: F) -> Callback<E>
    where
        W: Widget,
        F: Fn(&mut W, E) -> Result<()> + 'static,
        E: 'static,
    {
        widget_callback(WidgetId::<W>::new(self.common.id), func)
    }

    pub fn add_child(&mut self, widget: Box<dyn Widget>, options: LayoutItemOptions) -> usize {
        self.common.add_child(widget, options)
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
    let root_address = &root_widget.common().scope.address;

    if !address.starts_with(root_address) {
        warn!("get_widget_by_address_mut: address is not within root widget");
        return Err(WidgetNotFound);
    }
    let root_address_len = root_address.path.len();
    let mut current_widget = root_widget;
    for (index, _id) in &address.path[root_address_len..] {
        current_widget = current_widget
            .common_mut()
            .children
            .get_mut(*index)
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
    fn handle_widget_scope_change(&mut self, event: WidgetScopeChangeEvent) -> Result<()> {
        let _ = event;
        Ok(())
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
            Event::WidgetScopeChange(e) => self.handle_widget_scope_change(e).map(|()| true),
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

    fn with_no_padding(self, no_padding: bool) -> Self
    where
        Self: Sized;

    fn with_window(self, attrs: WindowAttributes) -> Self
    where
        Self: Sized;

    fn with_visible(self, value: bool) -> Self
    where
        Self: Sized;
    fn with_focusable(self, value: bool) -> Self
    where
        Self: Sized;
    fn with_accessible(self, value: bool) -> Self
    where
        Self: Sized;
    fn with_class(self, class: &'static str) -> Self
    where
        Self: Sized;
    fn with_pseudo_class(self, class: MyPseudoClass) -> Self
    where
        Self: Sized;

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
    fn set_scope(&mut self, scope: WidgetScope);
    fn set_enabled(&mut self, enabled: bool);
    fn set_visible(&mut self, visible: bool);
    fn set_style(&mut self, style: Option<Rc<Style>>) -> Result<()>;
    fn add_class(&mut self, class: &'static str);
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
    fn with_no_padding(mut self, no_padding: bool) -> Self
    where
        Self: Sized,
    {
        self.common_mut().set_no_padding(no_padding);
        self
    }

    fn with_window(mut self, attrs: WindowAttributes) -> Self
    where
        Self: Sized,
    {
        create_window(attrs, &mut self);
        self
    }

    fn with_visible(mut self, value: bool) -> Self
    where
        Self: Sized,
    {
        self.set_visible(value);
        self
    }
    fn with_focusable(mut self, value: bool) -> Self
    where
        Self: Sized,
    {
        self.common_mut().set_focusable(value);
        self
    }
    fn with_accessible(mut self, value: bool) -> Self
    where
        Self: Sized,
    {
        self.common_mut().set_accessible(value);
        self
    }

    fn with_class(mut self, class: &'static str) -> Self
    where
        Self: Sized,
    {
        self.add_class(class);
        self
    }
    fn with_pseudo_class(mut self, class: MyPseudoClass) -> Self
    where
        Self: Sized,
    {
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
                    for child in self.common_mut().children.iter_mut().rev() {
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
                    for child in self.common_mut().children.iter_mut().rev() {
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
                    for child in self.common_mut().children.iter_mut().rev() {
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
                for child in &mut self.common_mut().children {
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
                for child in &mut self.common_mut().children {
                    if let Some(rect_in_parent) = child.rect_in_parent {
                        if let Some(child_event) = event.map_to_child(rect_in_parent) {
                            child.widget.dispatch(child_event.into());
                        }
                    }
                }
            }
            Event::WindowFocusChange(event) => {
                for child in &mut self.common_mut().children {
                    child.widget.dispatch(event.clone().into());
                }
            }
            Event::WidgetScopeChange(_) => {
                self.common_mut().update_children_scope(0);
                self.common_mut().update();
            }
            Event::FocusIn(_) | Event::FocusOut(_) | Event::MouseLeave(_) => {
                self.common_mut().update();
            }
            Event::Layout(_) => {
                let len = self.common().children.len();
                for i in 0..len {
                    if !self.common().children[i].rect_set_during_layout {
                        let rect_in_parent = self.common().children[i].rect_in_parent;
                        self.common_mut()
                            .set_child_rect(i, rect_in_parent)
                            .or_report_err();
                    }
                    self.common_mut().children[i].rect_set_during_layout = false;
                }
                self.common_mut().current_layout_event = None;
                self.common_mut().update();
            }
            Event::ScrollToRect(event) => {
                if !accepted && event.address != self.common().scope.address {
                    if event.address.starts_with(&self.common().scope.address) {
                        if let Some((index, id)) =
                            event.address.item_at(self.common().scope.address.len())
                        {
                            if let Some(child) = self.common_mut().children.get_mut(index) {
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
                for child in &mut self.common_mut().children {
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
            Event::KeyboardInput(_)
            | Event::Ime(_)
            | Event::Accessible(_)
            | Event::StyleChange(_) => {}
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

        let Some(window) = self.common().scope.window.as_ref() else {
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

    fn set_scope(&mut self, scope: WidgetScope) {
        let previous_scope = self.common().scope.clone();
        self.common_mut().set_scope(scope);
        self.dispatch(WidgetScopeChangeEvent { previous_scope }.into());
        self.dispatch(StyleChangeEvent {}.into());
    }

    fn set_visible(&mut self, visible: bool) {
        if self.common_mut().is_self_visible == visible {
            return;
        }
        self.common_mut().is_self_visible = visible;
        self.common_mut().size_hint_changed(); // trigger layout
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

    fn add_class(&mut self, class: &'static str) {
        self.common_mut().style_element.add_class(class);
        self.dispatch(StyleChangeEvent {}.into());
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
        if pending_addr.starts_with(&common.scope.address) {
            common.clear_size_hint_cache();
            for child in &mut common.children {
                invalidate_size_hint_cache(child.widget.as_mut(), pending);
            }
            return;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WidgetAddress {
    pub path: Vec<(usize, RawWidgetId)>,
}

impl WidgetAddress {
    pub fn root(id: RawWidgetId) -> Self {
        Self {
            path: vec![(0, id)],
        }
    }
    pub fn join(mut self, index: usize, id: RawWidgetId) -> Self {
        self.path.push((index, id));
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
    pub fn strip_prefix(&self, parent: RawWidgetId) -> Option<&[(usize, RawWidgetId)]> {
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
    pub fn item_at(&self, pos: usize) -> Option<(usize, RawWidgetId)> {
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
