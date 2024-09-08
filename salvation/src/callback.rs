use std::{
    any::Any,
    collections::HashMap,
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::{anyhow, Context, Result};
use log::warn;
use winit::event_loop::EventLoopProxy;

use crate::{
    event_loop::{CallbackContext, UserEvent},
    system::{with_system, ReportError},
    widgets::{RawWidgetId, Widget, WidgetId},
};

#[derive(Debug, Clone, Copy)]
pub enum CallbackKind {
    State,
    Widget,
}

pub struct Callback<Event> {
    sender: EventLoopProxy<UserEvent>,
    callback_id: CallbackId,
    kind: CallbackKind,
    _marker: PhantomData<Event>,
}

impl<Event> Clone for Callback<Event> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            callback_id: self.callback_id,
            kind: self.kind,
            _marker: self._marker,
        }
    }
}

impl<Event> Callback<Event> {
    pub(crate) fn new(
        sender: EventLoopProxy<UserEvent>,
        callback_id: CallbackId,
        kind: CallbackKind,
    ) -> Self {
        Self {
            sender,
            callback_id,
            kind,
            _marker: PhantomData,
        }
    }
}

impl<Event: Send + 'static> Callback<Event> {
    pub fn invoke(&self, event: Event) {
        let event = UserEvent::InvokeCallback(InvokeCallbackEvent::new(
            self.callback_id,
            self.kind,
            Box::new(event),
        ));
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

pub type CallbackDataFn<State> =
    dyn FnMut(&mut State, &mut CallbackContext<State>, Box<dyn Any>) -> Result<()>;

struct CallbackData<State> {
    func: Box<CallbackDataFn<State>>,
    // TODO: weak ref for cleanup
}

pub struct CallbackMaker<State> {
    new_callbacks: Vec<(CallbackId, CallbackData<State>)>,
}

impl<State> CallbackMaker<State> {
    pub fn new() -> Self {
        Self {
            new_callbacks: Vec::new(),
        }
    }

    pub fn add(&mut self, callback: Box<CallbackDataFn<State>>) -> CallbackId {
        let callback_id = CallbackId::new();
        self.new_callbacks
            .push((callback_id, CallbackData { func: callback }));
        callback_id
    }
}

impl<State> Default for CallbackMaker<State> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Callbacks<State> {
    callbacks: HashMap<CallbackId, CallbackData<State>>,
}

impl<State> Callbacks<State> {
    pub fn new() -> Self {
        Self {
            callbacks: HashMap::new(),
        }
    }

    pub fn add_all(&mut self, maker: &mut CallbackMaker<State>) {
        self.callbacks.extend(maker.new_callbacks.drain(..));
    }

    pub fn call(
        &mut self,
        state: &mut State,
        ctx: &mut CallbackContext<State>,
        event: InvokeCallbackEvent,
    ) {
        if let Some(data) = self.callbacks.get_mut(&event.callback_id) {
            (data.func)(state, ctx, event.event).or_report_err();
        } else {
            warn!("unknown callback id");
        }
    }
}

impl<State> Default for Callbacks<State> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct InvokeCallbackEvent {
    pub callback_id: CallbackId,
    pub kind: CallbackKind,
    pub event: Box<dyn Any + Send>,
}

impl InvokeCallbackEvent {
    pub fn new(callback_id: CallbackId, kind: CallbackKind, event: Box<dyn Any + Send>) -> Self {
        Self {
            callback_id,
            kind,
            event,
        }
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
    Callback {
        sender: with_system(|s| s.event_loop_proxy.clone()),
        callback_id,
        kind: CallbackKind::Widget,
        _marker: PhantomData,
    }
}
