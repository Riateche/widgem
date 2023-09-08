use std::{any::Any, collections::HashMap, marker::PhantomData, rc::Rc};

use winit::event_loop::EventLoopProxy;

use crate::{
    event_loop::{CallbackContext, InvokeCallbackEvent, UserEvent},
    widgets::{RawWidgetId, Widget},
};

//pub type CallbackFn<State> = Rc<dyn FnMut(&mut State, Box<dyn Any>)>;

// pub struct InvokeCallbackEvent<State> {
//     func: CallbackFn<State>,
//     event: Box<dyn Any>,
// }

pub type CallbackFn<State, Event> = dyn Fn(&mut State, &mut CallbackContext<State>, Event);
pub struct Callback<Event> {
    pub sender: EventLoopProxy<UserEvent>,
    pub callback_id: CallbackId,
    pub _marker: PhantomData<Event>,
}

impl<Event: 'static> Callback<Event> {
    pub fn invoke(&self, event: Event) {
        let event = UserEvent::InvokeCallback(InvokeCallbackEvent {
            callback_id: self.callback_id,
            event: Box::new(event),
        });
        let _ = self.sender.send_event(event);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallbackId(u64);

pub type CallbackDataFn<State> = dyn FnMut(&mut State, &mut CallbackContext<State>, Box<dyn Any>);
pub struct CallbackData<State> {
    func: Box<CallbackDataFn<State>>,
    // TODO: weak ref for cleanup
}

pub struct CallbackMaker<State> {
    next_id: CallbackId,
    new_callbacks: Vec<(CallbackId, CallbackData<State>)>,
}

impl<State> CallbackMaker<State> {
    pub fn new() -> Self {
        Self {
            next_id: CallbackId(1),
            new_callbacks: Vec::new(),
        }
    }

    pub fn add(&mut self, callback: Box<CallbackDataFn<State>>) -> CallbackId {
        let callback_id = self.next_id;
        self.next_id.0 += 1;
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
            (data.func)(state, ctx, event.event);
        } else {
            println!("warning: unknown callback id");
        }
    }
}

impl<State> Default for Callbacks<State> {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct WidgetCallback<Event> {
    pub widget_id: RawWidgetId,
    pub func: Rc<dyn Fn(&mut dyn Widget, Event)>,
}
