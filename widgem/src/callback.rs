use {
    crate::{
        event_loop::UserEvent,
        system::with_system,
        widgets::{RawWidgetId, Widget, WidgetId},
    },
    anyhow::{anyhow, Context, Result},
    std::{
        any::Any,
        collections::HashMap,
        fmt,
        marker::PhantomData,
        rc::Rc,
        sync::atomic::{AtomicU64, Ordering},
    },
    winit::event_loop::EventLoopProxy,
};

#[must_use = "pass the `Callback` object to a `.on_...()` function of the sender widget to register the callback"]
pub struct Callback<Event> {
    sender: EventLoopProxy<UserEvent>,
    callback_id: CallbackId,
    widget_id: RawWidgetId,
    send_signals_on_setter_calls: bool,
    _marker: PhantomData<Event>,
}

impl<Event> Clone for Callback<Event> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            callback_id: self.callback_id,
            widget_id: self.widget_id,
            send_signals_on_setter_calls: self.send_signals_on_setter_calls,
            _marker: self._marker,
        }
    }
}

impl<Event> fmt::Debug for Callback<Event> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Callback")
            .field("sender", &self.sender)
            .field("callback_id", &self.callback_id)
            .field(
                "send_signals_on_setter_calls",
                &self.send_signals_on_setter_calls,
            )
            .finish()
    }
}

impl<Event> Callback<Event> {
    pub(crate) fn new(
        sender: EventLoopProxy<UserEvent>,
        callback_id: CallbackId,
        widget_id: RawWidgetId,
    ) -> Self {
        Self {
            sender,
            callback_id,
            widget_id,
            send_signals_on_setter_calls: false,
            _marker: PhantomData,
        }
    }

    // TODO: add example

    /// Configures when this callback should be invoked.
    ///
    /// If `enable == false` (default), the callback will only be invoked when the signal got triggered
    /// in response to a UI interaction (typically a mouse or keyboard event). The callback will not be invoked
    /// if the signal got triggered by a programmatic interaction (e.g. by calling a setter function).
    /// This default behavior is suitable in most cases and allows you to set up a two-way binding between
    /// the widget states.
    ///
    /// If `enable == true`, the callback will be invoked regardless of the way the signal got triggered.
    /// Use this option if you need to detect signals that got triggered programmatically (e.g. when
    /// some other widget called a setter function of the widget that provides the signal). Note that
    /// if you call a related setter in the callback, it can cause an infinite triggers of the callback.
    pub fn with_send_signals_on_setter_calls(mut self, enabled: bool) -> Self {
        self.send_signals_on_setter_calls = enabled;
        self
    }
}

impl<Event: Send + 'static> Callback<Event> {
    pub fn invoke(&self, event: Event) {
        let event =
            UserEvent::InvokeCallback(InvokeCallbackEvent::new(self.callback_id, Box::new(event)));
        let _ = self.sender.send_event(event);
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
        widget_id: widget_id.raw(),
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
    Callback::new(
        with_system(|s| s.event_loop_proxy.clone()),
        callback_id,
        widget_id.raw(),
    )
}

#[derive(Debug, Clone)]
pub struct Callbacks<Event> {
    // TODO: smallvec optimization?
    // TODO: remove callback after receiver is deleted
    callbacks: HashMap<RawWidgetId, Callback<Event>>,
}

impl<Event> Default for Callbacks<Event> {
    fn default() -> Self {
        Self {
            callbacks: HashMap::new(),
        }
    }
}

impl<Event> Callbacks<Event> {
    pub fn add(&mut self, callback: Callback<Event>) {
        self.callbacks.insert(callback.widget_id, callback);
    }

    pub fn invoke(&mut self, event: Event, from_setter: bool)
    where
        Event: Send + Clone + 'static,
    {
        for callback in self.callbacks.values() {
            if callback.send_signals_on_setter_calls || !from_setter {
                callback.invoke(event.clone());
            }
        }
    }
}
