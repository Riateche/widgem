use std::{any::Any, marker::PhantomData, rc::Rc};

use winit::event_loop::EventLoopProxy;

use crate::event_loop::{CallbackContext, InvokeCallbackEvent, UserEvent};

//pub type CallbackFn<State> = Rc<dyn FnMut(&mut State, Box<dyn Any>)>;

// pub struct InvokeCallbackEvent<State> {
//     func: CallbackFn<State>,
//     event: Box<dyn Any>,
// }

pub type CallbackFn<State, Event> = dyn Fn(&mut State, &mut CallbackContext<State>, Event);
pub struct Callback<State: 'static, Event> {
    func: Rc<CallbackFn<State, Event>>,
    sender: EventLoopProxy<UserEvent<State>>,
    _marker: PhantomData<Event>,
}

impl<State: 'static, Event: 'static> Callback<State, Event> {
    pub fn invoke(&self, event: Event) {
        let func = Rc::clone(&self.func);
        let event = UserEvent::InvokeCallback(InvokeCallbackEvent(Box::new(move |state, ctx| {
            func(state, ctx, event)
        })));
        let _ = self.sender.send_event(event);
    }
}

// struct CallbackId(u64);

// pub struct Callbacks<State> {
//     next_id: CallbackId,
//     callbacks: HashMap<CallbackId, Weak<dyn FnMut(&mut State, Box<dyn Any>)>>,
// }

// impl<State> Callbacks<State> {

// }
