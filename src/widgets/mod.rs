use std::{
    collections::HashMap,
    marker::PhantomData,
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
        UnmountEvent, WindowFocusChangeEvent,
    },
    layout::SizeHint,
    system::{address, register_address, send_window_request, unregister_address},
    types::{Rect, Size},
    window::{SetCursorIcon, SharedWindowData},
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

impl<T> Clone for WidgetId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for WidgetId<T> {}

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
}

#[derive(Debug, Clone)]
pub struct MountPoint {
    pub address: WidgetAddress,
    pub window: SharedWindowData,
    // Determines visual / accessible order.
    pub index_in_parent: usize,
    // TODO: separate event for updating index_in_parent without remounting
}

impl WidgetCommon {
    pub fn new() -> Self {
        Self {
            id: RawWidgetId::new(),
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
            mount_point.address.parent_widget().map(|id| id.into()),
            self.id.into(),
            mount_point.index_in_parent,
        );
        self.mount_point = Some(mount_point);
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
            mount_point.window.0.borrow_mut().accessible_nodes.unmount(
                mount_point.address.parent_widget().map(|id| id.into()),
                self.id.into(),
            );
        } else {
            warn!("widget was not mounted");
        }
        self.is_focused = false;
        self.is_window_focused = false;
    }

    pub fn size(&self) -> Option<Size> {
        self.rect_in_window.as_ref().map(|g| g.size)
    }

    pub fn add_child(&mut self, index: usize, mut widget: Box<dyn Widget>) {
        if let Some(mount_point) = &self.mount_point {
            let address = mount_point.address.clone().join(widget.common().id);
            widget.dispatch(
                MountEvent(MountPoint {
                    address,
                    window: mount_point.window.clone(),
                    index_in_parent: index,
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
        for i in index + 1..self.children.len() {
            self.children[i]
                .widget
                .common_mut()
                .update_index_in_parent(i);
        }
        self.size_hint_changed();
    }

    pub fn remove_child(&mut self, index: usize) -> Box<dyn Widget> {
        let mut widget = self.children.remove(index).widget;
        if self.mount_point.is_some() {
            widget.dispatch(UnmountEvent.into());
        }
        for i in index..self.children.len() {
            self.children[i]
                .widget
                .common_mut()
                .update_index_in_parent(i);
        }
        self.size_hint_changed();
        widget
    }

    fn update_index_in_parent(&mut self, index: usize) {
        if let Some(mount_point) = &mut self.mount_point {
            mount_point.index_in_parent = index;
            mount_point
                .window
                .0
                .borrow_mut()
                .accessible_nodes
                .update_index_in_parent(
                    mount_point.address.parent_widget().map(|id| id.into()),
                    self.id.into(),
                    index,
                );
        } else {
            warn!("update_index_in_parent: not mounted");
        }
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
    if address.path.get(0).copied() != Some(root_widget.common().id) {
        return Err(WidgetNotFound);
    }
    let mut current_widget = root_widget;
    for &id in &address.path[1..] {
        current_widget = current_widget
            .common_mut()
            .children
            .iter_mut()
            .find(|w| w.widget.common().id == id)
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

                for (i, child) in self.common_mut().children.iter_mut().enumerate() {
                    let child_address = mount_point.address.clone().join(child.widget.common().id);
                    child.widget.dispatch(
                        MountEvent(MountPoint {
                            address: child_address,
                            window: mount_point.window.clone(),
                            index_in_parent: i,
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
                        send_window_request(
                            mount_point.address.window_id,
                            SetCursorIcon(self.common().cursor_icon),
                        );
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
            }
            _ => {}
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
    pub path: Vec<RawWidgetId>,
}

impl WidgetAddress {
    pub fn window_root(window_id: WindowId) -> Self {
        Self {
            window_id,
            path: Vec::new(),
        }
    }
    pub fn join(mut self, id: RawWidgetId) -> Self {
        self.path.push(id);
        self
    }
    pub fn parent_widget(&self) -> Option<RawWidgetId> {
        if self.path.len() > 1 {
            Some(self.path[self.path.len() - 2])
        } else {
            None
        }
    }
    pub fn starts_with(&self, base: &WidgetAddress) -> bool {
        self.window_id == base.window_id
            && base.path.len() <= self.path.len()
            && base.path == self.path[..self.path.len()]
    }
}
