use std::{
    collections::HashMap,
    fmt::{self, Debug},
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use accesskit::NodeId;
use downcast_rs::{impl_downcast, Downcast};
use log::warn;
use winit::window::{CursorIcon, WindowId};

use crate::{
    draw::DrawEvent,
    event::{
        AccessibleEvent, CursorLeaveEvent, CursorMoveEvent, Event, FocusInEvent, FocusOutEvent,
        GeometryChangeEvent, ImeEvent, KeyboardInputEvent, MountEvent, MouseInputEvent,
        UnmountEvent, WidgetScopeChangeEvent, WindowFocusChangeEvent,
    },
    layout::SizeHint,
    style::Style,
    system::{address, register_address, unregister_address, with_system},
    types::{Rect, Size},
    window::SharedWindowData,
};

pub mod button;
pub mod column;
pub mod image;
pub mod label;
pub mod padding_box;
pub mod stack;
pub mod text_input;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawWidgetId(pub u64);

impl RawWidgetId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl From<RawWidgetId> for NodeId {
    fn from(value: RawWidgetId) -> Self {
        value.0.into()
    }
}

pub struct WidgetId<T>(pub RawWidgetId, pub PhantomData<T>);

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
    pub is_visible: bool,
    pub is_enabled: bool,
    pub style: Rc<Style>,
}

impl WidgetScope {
    pub fn root() -> Self {
        Self {
            is_visible: true,
            is_enabled: true,
            style: with_system(|s| s.style.clone()),
        }
    }
}

pub struct WidgetCommon {
    pub id: RawWidgetId,
    pub is_focusable: bool,
    pub enable_ime: bool,
    pub cursor_icon: CursorIcon,

    pub is_focused: bool,
    // TODO: set initial value in mount event
    pub is_window_focused: bool,

    pub is_mouse_entered: bool,
    pub mount_point: Option<MountPoint>,
    // Present if the widget is mounted, not hidden, and only after layout.
    pub rect_in_window: Option<Rect>,

    pub children: Vec<Child>,

    pub size_hint_x_cache: Option<SizeHint>,
    // TODO: limit count
    pub size_hint_y_cache: HashMap<i32, SizeHint>,

    pub pending_accessible_update: bool,

    pub is_explicitly_enabled: bool,
    pub is_explicitly_visible: bool,
    pub explicit_style: Option<Rc<Style>>,
}

#[derive(Debug, Clone)]
pub struct MountPoint {
    pub address: WidgetAddress,
    pub window: SharedWindowData,
    pub parent_id: Option<RawWidgetId>,
    // Determines visual / accessible order.
    // TODO: remove, use address
    pub index_in_parent: usize,
    pub parent_scope: WidgetScope,
}

impl WidgetCommon {
    pub fn new() -> Self {
        Self {
            id: RawWidgetId::new(),
            is_explicitly_enabled: true,
            is_explicitly_visible: true,
            explicit_style: None,
            is_focusable: false,
            is_focused: false,
            is_mouse_entered: false,
            is_window_focused: false,
            enable_ime: false,
            mount_point: None,
            rect_in_window: None,
            cursor_icon: CursorIcon::Default,
            children: Vec::new(),
            size_hint_x_cache: None,
            size_hint_y_cache: HashMap::new(),
            pending_accessible_update: false,
        }
    }

    //    let address = parent_address.join(self.id);

    pub fn mount(&mut self, mount_point: MountPoint) {
        if self.mount_point.is_some() {
            warn!("widget was already mounted");
        }
        let old = register_address(self.id, mount_point.address.clone());
        if old.is_some() {
            warn!("widget address was already registered");
        }
        mount_point.window.0.borrow_mut().widget_tree_changed = true;
        mount_point.window.0.borrow_mut().accessible_nodes.mount(
            mount_point.parent_id.map(|id| id.into()),
            self.id.into(),
            mount_point.index_in_parent,
        );
        self.mount_point = Some(mount_point);
        self.update();
        // TODO: set is_window_focused
    }

    pub fn unmount(&mut self) {
        if let Some(mount_point) = self.mount_point.take() {
            unregister_address(self.id);
            mount_point.window.0.borrow_mut().widget_tree_changed = true;
            mount_point
                .window
                .0
                .borrow_mut()
                .accessible_nodes
                .update(self.id.0.into(), None);
            mount_point
                .window
                .0
                .borrow_mut()
                .accessible_nodes
                .unmount(mount_point.parent_id.map(|id| id.into()), self.id.into());
        } else {
            warn!("widget was not mounted");
        }
        self.is_focused = false;
        self.is_window_focused = false;
    }

    pub fn is_visible(&self) -> bool {
        if let Some(mount_point) = &self.mount_point {
            mount_point.parent_scope.is_visible && self.is_explicitly_visible
        } else {
            self.is_explicitly_visible
        }
    }

    pub fn is_enabled(&self) -> bool {
        if let Some(mount_point) = &self.mount_point {
            mount_point.parent_scope.is_enabled && self.is_explicitly_enabled
        } else {
            self.is_explicitly_enabled
        }
    }

    pub fn style(&self) -> Rc<Style> {
        if let Some(mount_point) = &self.mount_point {
            self.explicit_style
                .as_ref()
                .unwrap_or(&mount_point.parent_scope.style)
                .clone()
        } else {
            with_system(|s| s.style.clone())
        }
    }

    pub fn effective_scope(&self) -> WidgetScope {
        WidgetScope {
            is_visible: self.is_visible(),
            is_enabled: self.is_enabled(),
            style: self.style(),
        }
    }

    pub fn size(&self) -> Option<Size> {
        self.rect_in_window.as_ref().map(|g| g.size)
    }

    // Request redraw and accessible update
    pub fn update(&mut self) {
        let Some(mount_point) = &self.mount_point else {
            return;
        };
        mount_point.window.request_redraw();
        self.pending_accessible_update = true;
    }

    pub fn add_child(&mut self, index: usize, mut widget: Box<dyn Widget>) {
        if let Some(mount_point) = &self.mount_point {
            let address = mount_point.address.clone().join(index);
            widget.dispatch(
                MountEvent(MountPoint {
                    address,
                    window: mount_point.window.clone(),
                    parent_id: Some(self.id),
                    index_in_parent: index,
                    parent_scope: self.effective_scope(),
                })
                .into(),
            );
        }
        self.children.insert(
            index,
            Child {
                widget,
                rect_in_parent: None,
            },
        );
        self.remount_children(index + 1);
        self.size_hint_changed();
    }

    fn remount_children(&mut self, from_index: usize) {
        let scope = self.effective_scope();
        if let Some(mount_point) = &self.mount_point {
            for i in from_index..self.children.len() {
                self.children[i].widget.dispatch(UnmountEvent.into());
                self.children[i].widget.dispatch(
                    MountEvent(MountPoint {
                        address: mount_point.address.clone().join(i),
                        window: mount_point.window.clone(),
                        parent_id: Some(self.id),
                        index_in_parent: i,
                        parent_scope: scope.clone(),
                    })
                    .into(),
                );
            }
        }
    }

    pub fn remove_child(&mut self, index: usize) -> Box<dyn Widget> {
        let mut widget = self.children.remove(index).widget;
        if self.mount_point.is_some() {
            widget.dispatch(UnmountEvent.into());
        }
        self.remount_children(index);
        self.size_hint_changed();
        widget
    }

    pub fn size_hint_changed(&mut self) {
        self.clear_size_hint_cache();
        let Some(mount_point) = &self.mount_point else {
            return;
        };
        mount_point
            .window
            .0
            .borrow_mut()
            .pending_size_hint_invalidations
            .push(mount_point.address.clone());
    }

    fn clear_size_hint_cache(&mut self) {
        self.size_hint_x_cache = None;
        self.size_hint_y_cache.clear();
    }
}

impl Default for WidgetCommon {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct WidgetNotFound;

pub fn get_widget_by_address_mut<'a>(
    root_widget: &'a mut dyn Widget,
    address: &WidgetAddress,
) -> Result<&'a mut dyn Widget, WidgetNotFound> {
    let mut current_widget = root_widget;
    for &index in &address.path {
        current_widget = current_widget
            .common_mut()
            .children
            .get_mut(index)
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
    pub rect_in_parent: Option<Rect>,
}

pub trait Widget: Downcast {
    fn common(&self) -> &WidgetCommon;
    fn common_mut(&mut self) -> &mut WidgetCommon;
    fn on_draw(&mut self, event: DrawEvent) {
        let _ = event;
    }
    fn on_mouse_input(&mut self, event: MouseInputEvent) -> bool {
        let _ = event;
        false
    }
    fn on_cursor_move(&mut self, event: CursorMoveEvent) -> bool {
        let _ = event;
        false
    }
    fn on_cursor_leave(&mut self, event: CursorLeaveEvent) {
        let _ = event;
    }
    fn on_keyboard_input(&mut self, event: KeyboardInputEvent) -> bool {
        let _ = event;
        false
    }
    fn on_ime(&mut self, event: ImeEvent) -> bool {
        let _ = event;
        false
    }
    // TODO: we don't need accept/reject for some event types
    fn on_geometry_change(&mut self, event: GeometryChangeEvent) {
        let _ = event;
    }
    fn on_widget_scope_change(&mut self, event: WidgetScopeChangeEvent) {
        let _ = event;
    }
    fn on_mount(&mut self, event: MountEvent) {
        let _ = event;
    }
    fn on_unmount(&mut self, event: UnmountEvent) {
        let _ = event;
    }
    fn on_focus_in(&mut self, event: FocusInEvent) {
        let _ = event;
    }
    fn on_focus_out(&mut self, event: FocusOutEvent) {
        let _ = event;
    }
    fn on_window_focus_change(&mut self, event: WindowFocusChangeEvent) {
        let _ = event;
    }
    fn on_accessible(&mut self, event: AccessibleEvent) {
        let _ = event;
    }
    fn on_event(&mut self, event: Event) -> bool {
        match event {
            Event::MouseInput(e) => self.on_mouse_input(e),
            Event::CursorMove(e) => self.on_cursor_move(e),
            Event::CursorLeave(e) => {
                self.on_cursor_leave(e);
                true
            }
            Event::KeyboardInput(e) => self.on_keyboard_input(e),
            Event::Ime(e) => self.on_ime(e),
            Event::Draw(e) => {
                self.on_draw(e);
                true
            }
            Event::GeometryChange(e) => {
                self.on_geometry_change(e);
                true
            }
            Event::Mount(e) => {
                self.on_mount(e);
                true
            }
            Event::Unmount(e) => {
                self.on_unmount(e);
                true
            }
            Event::FocusIn(e) => {
                self.on_focus_in(e);
                true
            }
            Event::FocusOut(e) => {
                self.on_focus_out(e);
                true
            }
            Event::WindowFocusChange(e) => {
                self.on_window_focus_change(e);
                true
            }
            Event::Accessible(e) => {
                self.on_accessible(e);
                true
            }
            Event::WidgetScopeChange(e) => {
                self.on_widget_scope_change(e);
                true
            }
        }
    }
    fn size_hint_x(&mut self) -> SizeHint;
    fn size_hint_y(&mut self, size_x: i32) -> SizeHint;

    #[must_use]
    fn layout(&mut self) -> Vec<Option<Rect>> {
        if !self.common().children.is_empty() {
            warn!("no layout impl for widget with children");
        }
        Vec::new()
    }
    fn accessible_node(&mut self) -> Option<accesskit::NodeBuilder> {
        None
    }
}
impl_downcast!(Widget);

pub trait WidgetExt {
    fn id(&self) -> WidgetId<Self>
    where
        Self: Sized;
    fn dispatch(&mut self, event: Event) -> bool;
    fn update_accessible(&mut self);
    fn apply_layout(&mut self);
    fn cached_size_hint_x(&mut self) -> SizeHint;
    fn cached_size_hint_y(&mut self, size_x: i32) -> SizeHint;

    // TODO: private
    fn set_parent_scope(&mut self, scope: WidgetScope);
    fn set_enabled(&mut self, enabled: bool);
    fn set_visible(&mut self, visible: bool);
    fn set_style(&mut self, style: Option<Rc<Style>>);
}

impl<W: Widget + ?Sized> WidgetExt for W {
    fn id(&self) -> WidgetId<Self>
    where
        Self: Sized,
    {
        WidgetId(self.common().id, PhantomData)
    }

    fn dispatch(&mut self, event: Event) -> bool {
        let mut accepted = false;
        match &event {
            Event::GeometryChange(event) => {
                self.common_mut().rect_in_window = event.new_rect_in_window;
            }
            Event::Mount(event) => {
                let mount_point = event.0.clone();
                self.common_mut().mount(mount_point.clone());

                let id = self.common().id;
                let scope = self.common().effective_scope();
                for (i, child) in self.common_mut().children.iter_mut().enumerate() {
                    let child_address = mount_point.address.clone().join(i);
                    child.widget.dispatch(
                        MountEvent(MountPoint {
                            address: child_address,
                            parent_id: Some(id),
                            window: mount_point.window.clone(),
                            index_in_parent: i,
                            parent_scope: scope.clone(),
                        })
                        .into(),
                    );
                }
            }
            // TODO: before or after handler?
            Event::Unmount(_event) => {
                for child in &mut self.common_mut().children {
                    child.widget.dispatch(UnmountEvent.into());
                }
            }
            Event::FocusIn(_) => {
                self.common_mut().is_focused = true;
            }
            Event::FocusOut(_) => {
                self.common_mut().is_focused = false;
            }
            Event::CursorLeave(_) => {
                self.common_mut().is_mouse_entered = false;
            }
            Event::WindowFocusChange(e) => {
                self.common_mut().is_window_focused = e.focused;
            }
            Event::MouseInput(event) => {
                for child in &mut self.common_mut().children {
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
            Event::CursorMove(event) => {
                for child in &mut self.common_mut().children {
                    if let Some(rect_in_parent) = child.rect_in_parent {
                        if rect_in_parent.contains(event.pos) {
                            let widget_id = child.widget.common().id;
                            let event = CursorMoveEvent {
                                pos: event.pos - rect_in_parent.top_left,
                                device_id: event.device_id,
                                accepted_by: event.accepted_by.clone(),
                                widget_id,
                                window: event.window.clone(),
                            };
                            if child.widget.dispatch(event.into()) {
                                accepted = true;
                                break;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        if !accepted {
            accepted = self.on_event(event.clone());
        }
        match event {
            Event::MouseInput(event) => {
                if event.accepted_by().is_none() && accepted {
                    event.set_accepted_by(self.common().id);
                }
            }
            Event::CursorMove(event) => {
                if event.accepted_by().is_none() && accepted {
                    if let Some(rect_in_window) = self.common().rect_in_window {
                        event.set_accepted_by(self.common().id, rect_in_window);
                        self.common_mut().is_mouse_entered = true;
                    } else {
                        warn!("no rect_in_window on CursorMove dispatch");
                    }
                    if let Some(mount_point) = &self.common().mount_point {
                        mount_point
                            .window
                            .0
                            .borrow()
                            .winit_window
                            .set_cursor_icon(self.common().cursor_icon);
                    }
                }
            }
            Event::Unmount(_) => {
                self.common_mut().unmount();
            }
            Event::Draw(event) => {
                for child in &mut self.common_mut().children {
                    if let Some(rect_in_parent) = child.rect_in_parent {
                        let child_event = event.map_to_child(rect_in_parent);
                        child.widget.dispatch(child_event.into());
                    }
                }
            }
            Event::WindowFocusChange(event) => {
                for child in &mut self.common_mut().children {
                    child.widget.dispatch(event.clone().into());
                }
            }
            Event::GeometryChange(_) => {
                self.apply_layout();
                self.common_mut().update();
            }
            Event::WidgetScopeChange(_) => {
                let scope = self.common().effective_scope();
                for child in &mut self.common_mut().children {
                    child.widget.as_mut().set_parent_scope(scope.clone());
                }
                self.common_mut().update();
            }
            Event::FocusIn(_) | Event::FocusOut(_) => {
                self.common_mut().update();
            }
            Event::CursorLeave(_)
            | Event::KeyboardInput(_)
            | Event::Ime(_)
            | Event::Mount(_)
            | Event::Accessible(_) => {}
        }

        self.update_accessible();
        accepted
    }

    fn apply_layout(&mut self) {
        let mut rects = self.layout();
        if rects.is_empty() {
            rects = self.common().children.iter().map(|_| None).collect();
        }
        if rects.len() != self.common().children.len() {
            warn!("invalid length in layout output");
            return;
        }
        let rect_in_window = self.common().rect_in_window;
        for (rect_in_parent, child) in rects.into_iter().zip(self.common_mut().children.iter_mut())
        {
            child.rect_in_parent = rect_in_parent;
            let child_rect_in_window = if let Some(rect_in_window) = rect_in_window {
                rect_in_parent
                    .map(|rect_in_parent| rect_in_parent.translate(rect_in_window.top_left))
            } else {
                None
            };
            if child.widget.common().rect_in_window != child_rect_in_window {
                child.widget.dispatch(
                    GeometryChangeEvent {
                        new_rect_in_window: child_rect_in_window,
                    }
                    .into(),
                );
            }
        }
    }

    fn update_accessible(&mut self) {
        if !self.common().pending_accessible_update {
            return;
        }
        let node = self.accessible_node();
        let Some(mount_point) = self.common().mount_point.as_ref() else {
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
        mount_point
            .window
            .0
            .borrow_mut()
            .accessible_nodes
            .update(self.common().id.0.into(), node);
        self.common_mut().pending_accessible_update = false;
    }

    fn cached_size_hint_x(&mut self) -> SizeHint {
        if let Some(cached) = &self.common().size_hint_x_cache {
            *cached
        } else {
            let r = self.size_hint_x();
            self.common_mut().size_hint_x_cache = Some(r);
            r
        }
    }
    fn cached_size_hint_y(&mut self, size_x: i32) -> SizeHint {
        if let Some(cached) = self.common().size_hint_y_cache.get(&size_x) {
            *cached
        } else {
            let r = self.size_hint_y(size_x);
            self.common_mut().size_hint_y_cache.insert(size_x, r);
            r
        }
    }

    fn set_parent_scope(&mut self, scope: WidgetScope) {
        if let Some(mount_point) = &mut self.common_mut().mount_point {
            mount_point.parent_scope = scope;
        } else {
            warn!("set_parent_scope: widget is not mounted");
        }
        self.dispatch(WidgetScopeChangeEvent.into());
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.common_mut().is_explicitly_enabled = enabled;
        self.dispatch(WidgetScopeChangeEvent.into());
    }

    fn set_visible(&mut self, visible: bool) {
        self.common_mut().is_explicitly_visible = visible;
        self.dispatch(WidgetScopeChangeEvent.into());
    }

    fn set_style(&mut self, style: Option<Rc<Style>>) {
        self.common_mut().explicit_style = style;
        self.dispatch(WidgetScopeChangeEvent.into());
    }
}

pub fn invalidate_size_hint_cache(widget: &mut dyn Widget, pending: &[WidgetAddress]) {
    let common = widget.common_mut();
    let Some(mount_point) = &common.mount_point else {
        return;
    };
    for pending_addr in pending {
        if pending_addr.starts_with(&mount_point.address) {
            common.clear_size_hint_cache();
            for child in &mut common.children {
                invalidate_size_hint_cache(child.widget.as_mut(), pending);
            }
            return;
        }
    }
}

#[derive(Debug, Clone)]
pub struct WidgetAddress {
    pub window_id: WindowId,
    pub path: Vec<usize>,
}

impl WidgetAddress {
    pub fn window_root(window_id: WindowId) -> Self {
        Self {
            window_id,
            path: Vec::new(),
        }
    }
    pub fn join(mut self, index: usize) -> Self {
        self.path.push(index);
        self
    }
    pub fn starts_with(&self, base: &WidgetAddress) -> bool {
        self.window_id == base.window_id
            && base.path.len() <= self.path.len()
            && base.path == self.path[..self.path.len()]
    }
}
