use std::{
    iter,
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use accesskit::NodeId;
use downcast_rs::{impl_downcast, Downcast};
use log::warn;
use winit::window::WindowId;

use crate::{
    draw::DrawEvent,
    event::{
        AccessibleEvent, CursorMovedEvent, Event, FocusInEvent, FocusOutEvent,
        GeometryChangedEvent, ImeEvent, KeyboardInputEvent, MountEvent, MouseInputEvent,
        UnmountEvent, WindowFocusChangedEvent,
    },
    system::{address, register_address, unregister_address},
    types::{Rect, Size},
    window::SharedWindowData,
};

pub mod button;
pub mod image;
pub mod label;
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

#[derive(Clone, Copy)]
pub struct Geometry {
    pub rect_in_window: Rect,
}

pub struct WidgetCommon {
    pub id: RawWidgetId,
    pub is_focusable: bool,
    pub enable_ime: bool,

    pub is_focused: bool,
    // TODO: set initial value in mount event
    pub is_window_focused: bool,
    pub mount_point: Option<MountPoint>,
    // Present if the widget is mounted, not hidden, and only after layout.
    pub geometry: Option<Geometry>,
}

#[derive(Clone)]
pub struct MountPoint {
    pub address: WidgetAddress,
    pub window: SharedWindowData,
    // Determines visual / accessible order.
    pub index_in_parent: i32,
    // TODO: separate event for updating index_in_parent without remounting
}

impl WidgetCommon {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            id: RawWidgetId::new(),
            is_focusable: false,
            is_focused: false,
            is_window_focused: false,
            enable_ime: false,
            mount_point: None,
            geometry: None,
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
        self.geometry.as_ref().map(|g| g.rect_in_window.size)
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
            .children_mut()
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
    pub index_in_parent: i32,
}

pub trait Widget: Downcast {
    fn common(&self) -> &WidgetCommon;
    fn common_mut(&mut self) -> &mut WidgetCommon;
    // TODO: why doesn't it work without Box?
    fn children_mut(&mut self) -> Box<dyn Iterator<Item = &mut Child> + '_> {
        Box::new(iter::empty())
    }
    fn on_draw(&mut self, ctx: DrawEvent);
    fn on_mouse_input(&mut self, event: MouseInputEvent) -> bool {
        let _ = event;
        false
    }
    fn on_cursor_moved(&mut self, event: CursorMovedEvent) -> bool {
        let _ = event;
        false
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
    fn on_geometry_changed(&mut self, event: GeometryChangedEvent) {
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
    fn on_window_focus_changed(&mut self, event: WindowFocusChangedEvent) {
        let _ = event;
    }
    fn on_accessible(&mut self, event: AccessibleEvent) {
        let _ = event;
    }
    fn on_event(&mut self, event: Event) -> bool {
        match event {
            Event::MouseInput(e) => self.on_mouse_input(e),
            Event::CursorMoved(e) => self.on_cursor_moved(e),
            Event::KeyboardInput(e) => self.on_keyboard_input(e),
            Event::Ime(e) => self.on_ime(e),
            Event::Draw(e) => {
                self.on_draw(e);
                true
            }
            Event::GeometryChanged(e) => {
                self.on_geometry_changed(e);
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
            Event::WindowFocusChanged(e) => {
                self.on_window_focus_changed(e);
                true
            }
            Event::Accessible(e) => {
                self.on_accessible(e);
                true
            }
        }
    }
    fn layout(&mut self) {}
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
}

impl<W: Widget + ?Sized> WidgetExt for W {
    fn id(&self) -> WidgetId<Self>
    where
        Self: Sized,
    {
        WidgetId(self.common().id, PhantomData)
    }

    fn dispatch(&mut self, event: Event) -> bool {
        let accepted_by = if let Event::MouseInput(mouse_input_event) = &event {
            Some(Rc::clone(&mouse_input_event.accepted_by))
        } else {
            None
        };
        let mut is_unmount = false;
        match &event {
            Event::GeometryChanged(event) => {
                self.common_mut().geometry = event.new_geometry;
            }
            Event::Mount(event) => {
                let mount_point = event.0.clone();
                self.common_mut().mount(mount_point.clone());
                for child in self.children_mut() {
                    let child_address = mount_point.address.clone().join(child.widget.common().id);
                    child.widget.dispatch(
                        MountEvent(MountPoint {
                            address: child_address,
                            window: mount_point.window.clone(),
                            index_in_parent: child.index_in_parent,
                        })
                        .into(),
                    );
                }
            }
            // TODO: before or after handler?
            Event::Unmount(_event) => {
                for child in self.children_mut() {
                    child.widget.dispatch(UnmountEvent.into());
                }
                is_unmount = true;
            }
            Event::FocusIn(_) => {
                self.common_mut().is_focused = true;
            }
            Event::FocusOut(_) => {
                self.common_mut().is_focused = false;
            }
            Event::WindowFocusChanged(e) => {
                self.common_mut().is_window_focused = e.focused;
            }
            _ => (),
        }
        let result = self.on_event(event);
        if let Some(accepted_by) = accepted_by {
            if accepted_by.get().is_none() && result {
                accepted_by.set(Some(self.common().id));
            }
        }
        if is_unmount {
            self.common_mut().unmount();
        }
        self.update_accessible();
        result
    }

    fn update_accessible(&mut self) {
        let node = self.accessible_node();
        let Some(mount_point) = self.common().mount_point.as_ref() else {
            return;
        };
        let geometry = self.common().geometry;
        let node = node.map(|mut node| {
            if let Some(geometry) = geometry {
                let rect = geometry.rect_in_window;
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
}
