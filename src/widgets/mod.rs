use std::{
    iter,
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

use downcast_rs::{impl_downcast, Downcast};
use winit::window::WindowId;

use crate::{
    draw::DrawEvent,
    event::{
        CursorMovedEvent, ImeEvent, KeyboardInputEvent, MouseInputEvent, ReceivedCharacterEvent, Event,
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

pub struct WidgetCommon {
    pub id: RawWidgetId,
    pub is_focusable: bool,
    pub mount_point: Option<MountPoint>,
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

pub fn mount(widget: &mut dyn Widget, mount_point: MountPoint) {
    widget.common_mut().mount(mount_point.clone());
    for child in widget.children_mut() {
        let child_address = mount_point.address.clone().join(child.common().id);
        mount(
            child.as_mut(),
            MountPoint {
                address: child_address,
                system: mount_point.system.clone(),
                window: mount_point.window.clone(),
            },
        );
    }
}

pub fn unmount(widget: &mut dyn Widget) {
    for child in widget.children_mut() {
        unmount(child.as_mut());
    }
    widget.common_mut().unmount();
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
    fn on_draw(&mut self, ctx: DrawEvent) -> bool;
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
    fn on_event(&mut self, event: Event) -> bool {
        match event {
            Event::MouseInput(e) => self.on_mouse_input(e),
            Event::CursorMoved(e) => self.on_cursor_moved(e),
            Event::KeyboardInput(e) => self.on_keyboard_input(e),
            Event::ReceivedCharacter(e) => self.on_received_character(e),
            Event::Ime(e) => self.on_ime(e),
            Event::Draw(e) => self.on_draw(e),
        }
    }
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
        Self: Sized {
        WidgetId(self.common().id, PhantomData)
    }

    fn dispatch(&mut self, event: Event) -> bool {
        self.on_event(event)
    }
}

pub struct Child {
    pub rect: Rect,
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
