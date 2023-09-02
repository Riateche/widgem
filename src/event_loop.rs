use std::{any::Any, cell::RefCell, collections::HashMap, fmt::Debug, marker::PhantomData, rc::Rc};

use cosmic_text::{FontSystem, SwashCache};
use scoped_tls::scoped_thread_local;
use tiny_skia::Color;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget},
    window::{WindowBuilder, WindowId},
};

use crate::{
    callback::{Callback, CallbackId, CallbackMaker, Callbacks},
    draw::Palette,
    widgets::{get_widget_by_address_mut, RawWidgetId, Widget, WidgetId, WidgetNotFound},
    window::{Window, WindowEventContext},
    SharedSystemData, SharedSystemDataInner,
};

type CallbackFn<State> = Box<dyn FnMut(&mut State, &mut CallbackContext<State>, Box<dyn Any>)>;
pub struct CallbackContext<'a, State> {
    pub windows: &'a mut HashMap<WindowId, Window>,
    pub add_callback: Box<dyn FnMut(CallbackFn<State>) -> CallbackId + 'a>,
    event_loop_proxy: EventLoopProxy<UserEvent>,
    shared_system_data: &'a SharedSystemData,
    //...
    marker: PhantomData<State>,
}

impl<'a, State> CallbackContext<'a, State> {
    pub fn map_state<AnotherState: 'static>(
        &mut self,
        mapper: impl Fn(&mut State) -> Option<&mut AnotherState> + Clone + 'static,
    ) -> CallbackContext<'_, AnotherState> {
        let add_callback = &mut self.add_callback;
        CallbackContext {
            windows: self.windows,
            shared_system_data: self.shared_system_data,
            event_loop_proxy: self.event_loop_proxy.clone(),
            marker: PhantomData,
            add_callback: Box::new(move |mut f| -> CallbackId {
                let mapper = mapper.clone();
                (add_callback)(Box::new(move |state, ctx, any_event| {
                    if let Some(another_state) = mapper(state) {
                        let mut new_ctx = ctx.map_state::<AnotherState>(mapper.clone());
                        f(another_state, &mut new_ctx, any_event)
                    }
                }))
            }),
        }
    }

    pub fn callback<Event: 'static>(
        &mut self,
        mut callback: impl FnMut(&mut State, &mut CallbackContext<State>, Event) + 'static,
    ) -> Callback<Event> {
        let callback_id = (self.add_callback)(Box::new(move |state, ctx, any_event| {
            let event = *any_event
                .downcast::<Event>()
                .expect("event downcast failed");
            callback(state, ctx, event);
        }));

        Callback {
            sender: self.event_loop_proxy.clone(),
            callback_id,
            _marker: PhantomData,
        }
    }

    // TODO: create builder instead
    pub fn add_window(&mut self, title: &str, root_widget: Option<Box<dyn Widget>>) -> WindowId {
        let builder = WindowBuilder::new().with_title(title);
        let window = WINDOW_TARGET.with(|window_target| builder.build(window_target).unwrap());
        let window = Window::new(window, self.shared_system_data.clone(), root_widget);
        let id = window.inner.id();
        self.windows.insert(id, window);
        id
    }

    pub fn get_widget_by_id_mut<W: Widget>(
        &mut self,
        id: WidgetId<W>,
    ) -> Result<&mut W, WidgetNotFound> {
        let w = self.get_widget_by_raw_id_mut(id.0)?;
        Ok(w.downcast_mut::<W>().expect("widget downcast failed"))
    }

    pub fn get_widget_by_raw_id_mut(
        &mut self,
        id: RawWidgetId,
    ) -> Result<&mut dyn Widget, WidgetNotFound> {
        let address = self
            .shared_system_data
            .0
            .borrow()
            .address_book
            .get(&id)
            .ok_or(WidgetNotFound)?
            .clone();
        let window = self
            .windows
            .get_mut(&address.window_id)
            .ok_or(WidgetNotFound)?;
        let widget = window.root_widget.as_mut().ok_or(WidgetNotFound)?;
        get_widget_by_address_mut(widget.as_mut(), &address)
    }
}

#[derive(Debug)]
pub struct InvokeCallbackEvent {
    pub callback_id: CallbackId,
    pub event: Box<dyn Any>,
}

#[derive(Debug)]
pub enum UserEvent {
    InvokeCallback(InvokeCallbackEvent),
}

scoped_thread_local!(pub static WINDOW_TARGET: EventLoopWindowTarget<UserEvent>);

pub fn run<State: 'static>(make_state: impl FnOnce(&mut CallbackContext<State>) -> State) {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    let mut windows = HashMap::new();

    let shared_system_data = SharedSystemData(Rc::new(RefCell::new(SharedSystemDataInner {
        address_book: HashMap::new(),
        font_system: FontSystem::new(),
        swash_cache: SwashCache::new(),
        font_metrics: cosmic_text::Metrics::new(24.0, 30.0),
        palette: Palette {
            foreground: Color::BLACK,
            background: Color::WHITE,
            // foreground: Color::WHITE,
            // background: Color::BLACK,
        },
    })));

    let mut callback_maker = CallbackMaker::<State>::new();
    let mut callbacks = Callbacks::<State>::new();

    let event_loop_proxy = event_loop.create_proxy();

    let mut state = {
        let mut ctx = CallbackContext {
            windows: &mut windows,
            shared_system_data: &shared_system_data,
            add_callback: Box::new(|f| callback_maker.add(f)),
            marker: PhantomData,
            event_loop_proxy: event_loop_proxy.clone(),
        };
        WINDOW_TARGET.set(&event_loop, || make_state(&mut ctx))
    };
    callbacks.add_all(&mut callback_maker);

    event_loop.run(move |event, window_target, control_flow| {
        WINDOW_TARGET.set(window_target, || {
            *control_flow = ControlFlow::Wait;

            let mut ctx = WindowEventContext {};

            match event {
                Event::RedrawRequested(window_id) => {
                    if let Some(window) = windows.get_mut(&window_id) {
                        window.handle_event(&mut ctx, event.map_nonuser_event().unwrap());
                    }
                }
                Event::WindowEvent {
                    window_id,
                    event: ref wevent,
                } => {
                    // TODO: smarter logic
                    if matches!(wevent, WindowEvent::CloseRequested) {
                        *control_flow = ControlFlow::Exit;
                    }
                    if let Some(window) = windows.get_mut(&window_id) {
                        window.handle_event(&mut ctx, event.map_nonuser_event().unwrap());
                    }
                }
                Event::UserEvent(event) => match event {
                    UserEvent::InvokeCallback(event) => {
                        {
                            let mut ctx = CallbackContext {
                                windows: &mut windows,
                                shared_system_data: &shared_system_data,
                                add_callback: Box::new(|f| callback_maker.add(f)),
                                marker: PhantomData,
                                event_loop_proxy: event_loop_proxy.clone(),
                            };

                            callbacks.call(&mut state, &mut ctx, event);
                        }
                        callbacks.add_all(&mut callback_maker);
                    }
                },
                _ => {}
            }
        });
    });
}
