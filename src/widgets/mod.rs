use std::{
    iter,
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

use downcast_rs::{impl_downcast, Downcast};
use winit::window::WindowId;

use crate::{
    draw::DrawContext,
    event::{
        CursorMovedEvent, ImeEvent, KeyboardInputEvent, MouseInputEvent, ReceivedCharacterEvent,
    },
    types::Rect,
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
    pub address: Option<WidgetAddress>,
    pub system: Option<SharedSystemData>,
}

impl WidgetCommon {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            id: RawWidgetId::new(),
            is_focusable: false,
            system: None,
            address: None,
        }
    }

    pub fn mount(&mut self, data: SharedSystemData, parent_address: WidgetAddress) {
        if self.system.is_some() || self.address.is_some() {
            println!("warn: widget was already mounted");
        }
        let address = parent_address.join(self.id);
        let old = data
            .0
            .borrow_mut()
            .address_book
            .insert(self.id, address.clone());
        if old.is_some() {
            println!("warn: widget address was already registered");
        }
        data.0
            .borrow_mut()
            .widget_tree_changed_flags
            .insert(address.window_id);
        self.address = Some(address);
        self.system = Some(data);
    }

    pub fn unmount(&mut self) {
        if let (Some(system), Some(address)) = (&self.system, &self.address) {
            system.0.borrow_mut().address_book.remove(&self.id);
            system
                .0
                .borrow_mut()
                .widget_tree_changed_flags
                .insert(address.window_id);
        } else {
            println!("warn: widget was not mounted");
        }
        self.system = None;
        self.address = None;
    }
}

pub fn mount(widget: &mut dyn Widget, data: SharedSystemData, parent_address: WidgetAddress) {
    widget
        .common_mut()
        .mount(data.clone(), parent_address.clone());
    let address = widget.common().address.clone().expect("already mounted");
    for child in widget.children_mut() {
        mount(child.as_mut(), data.clone(), address.clone());
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
    fn draw(&mut self, ctx: &mut DrawContext<'_>);
    fn mouse_input(&mut self, event: &mut MouseInputEvent<'_>) {
        let _ = event;
    }
    fn cursor_moved(&mut self, event: &mut CursorMovedEvent<'_>) {
        let _ = event;
    }
    fn keyboard_input(&mut self, event: &mut KeyboardInputEvent) {
        let _ = event;
    }
    fn received_character(&mut self, event: &mut ReceivedCharacterEvent) {
        let _ = event;
    }
    fn ime(&mut self, event: &mut ImeEvent) {
        let _ = event;
    }
}
impl_downcast!(Widget);

pub trait WidgetExt {
    fn id(&self) -> WidgetId<Self>
    where
        Self: Sized;
}

impl<W: Widget> WidgetExt for W {
    fn id(&self) -> WidgetId<Self> {
        WidgetId(self.common().id, PhantomData)
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
