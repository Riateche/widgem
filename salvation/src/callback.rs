use std::{
    any::Any,
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::{anyhow, Context, Result};
use winit::event_loop::EventLoopProxy;

use crate::{
    event_loop::UserEvent,
    system::with_system,
    widgets::{RawWidgetId, Widget, WidgetId},
};

pub struct Callback<Event> {
    sender: EventLoopProxy<UserEvent>,
    callback_id: CallbackId,
    _marker: PhantomData<Event>,
}

impl<Event> Clone for Callback<Event> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            callback_id: self.callback_id,
            _marker: self._marker,
        }
    }
}

impl<Event> Callback<Event> {
    pub(crate) fn new(sender: EventLoopProxy<UserEvent>, callback_id: CallbackId) -> Self {
        Self {
            sender,
            callback_id,
            _marker: PhantomData,
        }
    }
}

impl<Event: Send + 'static> Callback<Event> {
    pub fn invoke(&self, event: Event) {
        let event =
            UserEvent::InvokeCallback(InvokeCallbackEvent::new(self.callback_id, Box::new(event)));
        let _ = self.sender.send_event(event);
    }
}

pub struct CallbackVec<Event>(Vec<Callback<Event>>);

impl<Event> CallbackVec<Event> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, callback: Callback<Event>) {
        self.0.push(callback);
    }

    pub fn invoke(&self, event: Event)
    where
        Event: Send + Clone + 'static,
    {
        for item in &self.0 {
            item.invoke(event.clone());
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallbackId(u64);

impl CallbackId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug)]
pub struct InvokeCallbackEvent {
    pub callback_id: CallbackId,
    pub event: Box<dyn Any + Send>,
}

impl InvokeCallbackEvent {
    pub fn new(callback_id: CallbackId, event: Box<dyn Any + Send>) -> Self {
        Self { callback_id, event }
    }
}

pub type WidgetCallbackDataFn = dyn Fn(&mut dyn Widget, Box<dyn Any + Send>) -> Result<()>;

#[derive(Clone)]
pub struct WidgetCallbackData {
    pub widget_id: RawWidgetId,
    pub func: Rc<WidgetCallbackDataFn>,
    // TODO: weak ref for cleanup
}

pub fn widget_callback<W, E, F>(widget_id: WidgetId<W>, func: F) -> Callback<E>
where
    W: Widget,
    F: Fn(&mut W, E) -> Result<()> + 'static,
    E: 'static,
{
    let callback_id = CallbackId::new();
    let data = WidgetCallbackData {
        widget_id: widget_id.0,
        func: Rc::new(move |widget, any_event| {
            let widget = widget
                .downcast_mut::<W>()
                .context("widget downcast failed")?;
            let event = any_event
                .downcast::<E>()
                .map_err(|_| anyhow!("event downcast failed"))?;
            func(widget, *event)
        }),
    };
    with_system(|s| s.widget_callbacks.insert(callback_id, data));
    Callback::new(with_system(|s| s.event_loop_proxy.clone()), callback_id)
}
