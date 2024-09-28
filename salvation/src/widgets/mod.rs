use std::{
    cell::Cell,
    collections::{BTreeMap, HashMap},
    fmt::{self, Debug},
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use accesskit::NodeId;
use anyhow::{bail, Context, Result};
use downcast_rs::{impl_downcast, Downcast};
use log::warn;
use thiserror::Error;
use winit::window::{CursorIcon, WindowAttributes, WindowId};

use crate::{
    callback::{widget_callback, Callback},
    create_window,
    draw::DrawEvent,
    event::{
        AccessibleActionEvent, Event, FocusInEvent, FocusOutEvent, ImeEvent, KeyboardInputEvent,
        LayoutEvent, MouseEnterEvent, MouseInputEvent, MouseLeaveEvent, MouseMoveEvent,
        WidgetScopeChangeEvent, WindowFocusChangeEvent,
    },
    layout::{
        grid::{self, GridAxisOptions, GridOptions},
        LayoutItemOptions, SizeHintMode, SizeHints, FALLBACK_SIZE_HINT,
    },
    shortcut::{Shortcut, ShortcutId, ShortcutScope},
    style::{computed::ComputedStyle, Style},
    system::{address, register_address, unregister_address, with_system, ReportError},
    types::{Point, Rect, Size},
    window::Window,
};

pub mod button;
pub mod column;
pub mod image;
pub mod label;
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

    pub is_visible: bool,
    pub is_enabled: bool,
    pub style: Rc<ComputedStyle>,
}

impl WidgetScope {
    pub fn window_id(&self) -> Option<WindowId> {
        self.window.as_ref().map(|w| w.0.borrow().id)
    }
}

impl WidgetScope {
    fn new(id: RawWidgetId) -> Self {
        with_system(|s| Self {
            parent_id: None,
            address: WidgetAddress::root(id),
            window: None,
            is_visible: true,
            is_enabled: true,
            style: s.default_style.clone(),
        })
    }
}

pub type EventFilterFn = dyn Fn(Event) -> Result<bool>;

pub struct WidgetCommon {
    pub id: RawWidgetId,
    pub is_focusable: bool,
    pub enable_ime: bool,
    pub cursor_icon: CursorIcon,

    pub is_focused: bool,
    // TODO: set initial value in mount event
    pub is_window_focused: bool,
    pub scope: WidgetScope,
    pub is_window_root: bool,

    pub is_mouse_over: bool,
    // Present if the widget is mounted, not hidden, and only after layout.
    pub rect_in_window: Option<Rect>,

    pub children: Vec<Child>,
    pub current_layout_event: Option<LayoutEvent>,

    pub size_hint_x_cache: HashMap<SizeHintMode, i32>,
    // TODO: limit count
    pub size_hint_y_cache: HashMap<(i32, SizeHintMode), i32>,
    pub size_x_fixed_cache: Option<bool>,
    pub size_y_fixed_cache: Option<bool>,

    pub pending_accessible_update: bool,

    pub is_explicitly_enabled: bool,
    pub is_explicitly_visible: bool,
    pub explicit_style: Option<Rc<ComputedStyle>>,

    pub is_registered_as_focusable: bool,
    // TODO: multiple filters?
    // TODO: accept/reject event from filter; option to run filter after on_event
    pub event_filter: Option<Box<EventFilterFn>>,
    pub accessible_mounted: bool,
    pub grid_options: Option<GridOptions>,

    pub shortcuts: Vec<Shortcut>,
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
    pub fn new() -> Self {
        let id = RawWidgetId::new();
        let scope = WidgetScope::new(id);
        register_address(id, scope.address.clone());

        Self {
            id,
            is_explicitly_enabled: true,
            is_explicitly_visible: true,
            explicit_style: None,
            is_focusable: false,
            is_focused: false,
            is_mouse_over: false,
            is_window_focused: false,
            enable_ime: false,
            rect_in_window: None,
            cursor_icon: CursorIcon::Default,
            children: Vec::new(),
            size_hint_x_cache: HashMap::new(),
            size_hint_y_cache: HashMap::new(),
            size_x_fixed_cache: None,
            size_y_fixed_cache: None,
            pending_accessible_update: false,
            scope,
            is_registered_as_focusable: false,
            event_filter: None,
            current_layout_event: None,
            is_window_root: false,
            accessible_mounted: false,
            grid_options: None,
            shortcuts: Vec::new(),
        }
    }

    pub fn set_grid_options(&mut self, options: Option<GridOptions>) {
        self.grid_options = options;
        self.size_hint_changed();
    }

    fn grid_options(&self) -> GridOptions {
        self.grid_options.clone().unwrap_or_else(|| {
            let style = self.style();
            GridOptions {
                x: GridAxisOptions {
                    min_padding: style.grid.min_padding.x,
                    min_spacing: style.grid.min_spacing.x,
                    preferred_padding: style.grid.preferred_padding.x,
                    preferred_spacing: style.grid.preferred_spacing.x,
                    border_collapse: 0,
                },
                y: GridAxisOptions {
                    min_padding: style.grid.min_padding.y,
                    min_spacing: style.grid.min_spacing.y,
                    preferred_padding: style.grid.preferred_padding.y,
                    preferred_spacing: style.grid.preferred_spacing.y,
                    border_collapse: 0,
                },
            }
        })
    }

    pub fn is_visible(&self) -> bool {
        self.scope.is_visible && self.is_explicitly_visible
    }

    pub fn is_enabled(&self) -> bool {
        self.scope.is_enabled && self.is_explicitly_enabled
    }

    pub fn is_focusable(&self) -> bool {
        self.is_focusable && self.is_enabled()
    }

    pub fn is_focused(&self) -> bool {
        self.is_focused && self.is_window_focused
    }

    pub fn style(&self) -> &Rc<ComputedStyle> {
        self.explicit_style.as_ref().unwrap_or(&self.scope.style)
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
            is_visible: self.is_visible(),
            is_enabled: self.is_enabled(),
            // TODO: allow overriding scale?
            style: self.style().clone(),
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
        child.rect_in_parent = rect_in_parent;
        let rect_changed = child.widget.common().rect_in_window != rect_in_window;
        if let Some(event) = &self.current_layout_event {
            if rect_changed || event.size_hints_changed_within(child.widget.common().address()) {
                child.widget.dispatch(
                    LayoutEvent::new(rect_in_window, event.changed_size_hints.clone()).into(),
                );
            }
            child.rect_set_during_layout = true;
        } else {
            if rect_changed {
                child
                    .widget
                    .dispatch(LayoutEvent::new(rect_in_window, Vec::new()).into());
            }
        }
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
        window
            .0
            .borrow_mut()
            .pending_size_hint_invalidations
            .push(self.scope.address.clone());
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

    pub fn set_enabled(&mut self, enabled: bool) {
        self.is_explicitly_enabled = enabled;
        self.register_focusable();
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
            let root_widget_id = window.0.borrow().root_widget_id;
            window.0.borrow_mut().accessible_nodes.unmount(
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
                    window
                        .0
                        .borrow_mut()
                        .accessible_nodes
                        .update(self.id.0.into(), None);
                }
            }
        }

        self.scope = scope;

        if addr_changed {
            register_address(self.id, self.scope.address.clone());
        }
        if update_accessible {
            if let Some(window) = &self.scope.window {
                let root_widget_id = window.0.borrow().root_widget_id;
                window.0.borrow_mut().accessible_nodes.mount(
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
        self.update();
        self.register_focusable();
        // TODO: set is_window_focused
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
}

impl Default for WidgetCommon {
    fn default() -> Self {
        Self::new()
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

pub struct Child {
    pub widget: Box<dyn Widget>,
    pub options: LayoutItemOptions,
    pub rect_in_parent: Option<Rect>,
    pub rect_set_during_layout: bool,
}

pub trait Widget: Downcast {
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
    fn handle_event(&mut self, event: Event) -> Result<bool> {
        match event {
            Event::MouseInput(e) => self.handle_mouse_input(e),
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

    fn with_window(self, attrs: WindowAttributes) -> Self
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
    fn set_style(&mut self, style: Option<Style>) -> Result<()>;

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

    fn with_window(mut self, attrs: WindowAttributes) -> Self
    where
        Self: Sized,
    {
        create_window(attrs, &mut self);
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
            }
            Event::FocusOut(_) => {
                self.common_mut().is_focused = false;
            }
            Event::WindowFocusChange(e) => {
                self.common_mut().is_window_focused = e.is_focused();
            }
            Event::MouseInput(event) => {
                should_dispatch = self.common().is_enabled();
                if should_dispatch {
                    for child in self.common_mut().children.iter_mut().rev() {
                        if let Some(rect_in_parent) = child.rect_in_parent {
                            if let Some(child_event) = event.map_to_child(rect_in_parent) {
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
                            if let Some(child_event) = event.map_to_child(rect_in_parent) {
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
                                let self_id = self.common().id;
                                !window
                                    .0
                                    .borrow()
                                    .mouse_entered_widgets
                                    .iter()
                                    .any(|(_, id)| *id == self_id)
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
                should_dispatch = self.common().is_enabled();
            }
            Event::Layout(event) => {
                self.common_mut().rect_in_window = event.new_rect_in_window;
                self.common_mut().current_layout_event = Some(event.clone());
                for child in &mut self.common_mut().children {
                    child.rect_set_during_layout = false;
                }
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
            Event::MouseInput(event) => {
                if event.accepted_by.get().is_none() && accepted {
                    event.accepted_by.set(Some(self.common().id));
                }
            }
            Event::MouseEnter(event) => {
                accept_mouse_event(self, true, &event.accepted_by);
            }
            Event::MouseMove(event) => {
                accept_mouse_event(self, false, &event.accepted_by);
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
            Event::KeyboardInput(_) | Event::Ime(_) | Event::Accessible(_) => {}
        }

        self.update_accessible();
        accepted
    }

    fn update_accessible(&mut self) {
        if !self.common().pending_accessible_update {
            return;
        }
        let node = self.accessible_node();
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
        window
            .0
            .borrow_mut()
            .accessible_nodes
            .update(self.common().id.0.into(), node);
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
        self.dispatch(WidgetScopeChangeEvent::new(previous_scope).into());
    }

    fn set_enabled(&mut self, enabled: bool) {
        let previous_scope = self.common().scope.clone();
        self.common_mut().set_enabled(enabled);
        self.dispatch(WidgetScopeChangeEvent::new(previous_scope).into());
    }

    fn set_visible(&mut self, visible: bool) {
        let previous_scope = self.common().scope.clone();
        self.common_mut().is_explicitly_visible = visible;
        self.dispatch(WidgetScopeChangeEvent::new(previous_scope).into());
    }

    fn set_style(&mut self, style: Option<Style>) -> Result<()> {
        let previous_scope = self.common().scope.clone();
        let scale = self.common().scope.style.scale;
        let style = if let Some(style) = style {
            Some(Rc::new(ComputedStyle::new(&style, scale)?))
        } else {
            None
        };
        self.common_mut().explicit_style = style;
        self.dispatch(WidgetScopeChangeEvent::new(previous_scope).into());
        Ok(())
    }

    fn boxed(self) -> Box<dyn Widget>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

fn accept_mouse_event(
    widget: &mut (impl Widget + ?Sized),
    is_enter: bool,
    accepted_by: &Rc<Cell<Option<RawWidgetId>>>,
) {
    if accepted_by.get().is_none() {
        let Some(rect_in_window) = widget.common().rect_in_window_or_err().or_report_err() else {
            return;
        };
        let Some(window) = widget.common().window_or_err().or_report_err() else {
            return;
        };
        let id = widget.common().id;
        accepted_by.set(Some(id));

        window
            .0
            .borrow()
            .winit_window
            .set_cursor(widget.common().cursor_icon);
        if is_enter {
            window
                .0
                .borrow_mut()
                .mouse_entered_widgets
                .push((rect_in_window, id));

            widget.common_mut().is_mouse_over = true;
            widget.common_mut().update();
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
}

#[macro_export]
macro_rules! impl_widget_common {
    () => {
        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }
    };
}
