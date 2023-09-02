use std::{
    iter,
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use downcast_rs::{impl_downcast, Downcast};
use winit::window::WindowId;

use crate::{
    draw::DrawEvent,
    event::{
        CursorMovedEvent, Event, GeometryChangedEvent, ImeEvent, KeyboardInputEvent, MountEvent,
        MouseInputEvent, ReceivedCharacterEvent, UnmountEvent,
    },
    types::Rect,
    window::SharedWindowData,
    SharedSystemData,
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

pub struct WidgetId<T>(pub RawWidgetId, pub PhantomData<T>);

impl<T> Clone for WidgetId<T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
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
    pub mount_point: Option<MountPoint>,
    // Present if the widget is mounted, not hidden, and only after layout.
    pub geometry: Option<Geometry>,
}

#[derive(Clone)]
pub struct MountPoint {
    pub address: WidgetAddress,
    pub system: SharedSystemData,
    pub window: SharedWindowData,
}

impl WidgetCommon {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            id: RawWidgetId::new(),
            is_focusable: false,
            mount_point: None,
            geometry: None,
        }
    }

    //    let address = parent_address.join(self.id);

    pub fn mount(&mut self, mount_point: MountPoint) {
        if self.mount_point.is_some() {
            println!("warn: widget was already mounted");
        }
        let old = mount_point
            .system
            .0
            .borrow_mut()
            .address_book
            .insert(self.id, mount_point.address.clone());
        if old.is_some() {
            println!("warn: widget address was already registered");
        }
        mount_point.window.0.borrow_mut().widget_tree_changed = true;
        self.mount_point = Some(mount_point);
    }

    pub fn unmount(&mut self) {
        if let Some(mount_point) = self.mount_point.take() {
            mount_point
                .system
                .0
                .borrow_mut()
                .address_book
                .remove(&self.id);
            mount_point.window.0.borrow_mut().widget_tree_changed = true;
        } else {
            println!("warn: widget was not mounted");
        }
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
            .find(|w| w.common().id == id)
            .ok_or(WidgetNotFound)?
            .as_mut();
    }
    Ok(current_widget)
}

pub trait Widget: Downcast {
    fn common(&self) -> &WidgetCommon;
    fn common_mut(&mut self) -> &mut WidgetCommon;
    // TODO: why doesn't it work without Box?
    fn children_mut(&mut self) -> Box<dyn Iterator<Item = &mut Box<dyn Widget>> + '_> {
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
    fn on_received_character(&mut self, event: ReceivedCharacterEvent) -> bool {
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
    fn on_event(&mut self, event: Event) -> bool {
        match event {
            Event::MouseInput(e) => self.on_mouse_input(e),
            Event::CursorMoved(e) => self.on_cursor_moved(e),
            Event::KeyboardInput(e) => self.on_keyboard_input(e),
            Event::ReceivedCharacter(e) => self.on_received_character(e),
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
        }
    }
    fn layout(&mut self) {}
}
impl_downcast!(Widget);

pub trait WidgetExt {
    fn id(&self) -> WidgetId<Self>
    where
        Self: Sized;
    fn dispatch(&mut self, event: Event) -> bool;
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
        match &event {
            Event::GeometryChanged(event) => {
                self.common_mut().geometry = event.new_geometry;
            }
            Event::Mount(event) => {
                let mount_point = event.0.clone();
                self.common_mut().mount(mount_point.clone());
                for child in self.children_mut() {
                    let child_address = mount_point.address.clone().join(child.common().id);
                    child.dispatch(
                        MountEvent(MountPoint {
                            address: child_address,
                            system: mount_point.system.clone(),
                            window: mount_point.window.clone(),
                        })
                        .into(),
                    );
                }
            }
            // TODO: before or after handler?
            Event::Unmount(_event) => {
                for child in self.children_mut() {
                    child.dispatch(UnmountEvent.into());
                }
                self.common_mut().unmount();
            }
            _ => (),
        }
        let result = self.on_event(event);
        if let Some(accepted_by) = accepted_by {
            if accepted_by.get().is_none() && result {
                accepted_by.set(Some(self.common().id));
            }
        }
        result
    }
}

pub struct Child {
    pub rect_in_parent: Rect,
    pub widget: Box<dyn Widget>,
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
}
