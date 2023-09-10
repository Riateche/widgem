use std::{any::Any, collections::HashMap, fmt::Debug, marker::PhantomData, time::Instant};

use accesskit_winit::ActionRequestEvent;
use arboard::Clipboard;
use cosmic_text::{FontSystem, SwashCache};
use derive_more::From;
use scoped_tls::scoped_thread_local;
use tiny_skia::Color;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget},
    window::{WindowBuilder, WindowId},
};

use crate::{
    callback::{Callback, CallbackId, CallbackMaker, Callbacks, WidgetCallback},
    draw::Palette,
    system::{address, with_system, SharedSystemDataInner, SYSTEM},
    timer::Timers,
    widgets::{get_widget_by_address_mut, RawWidgetId, Widget, WidgetId, WidgetNotFound},
    window::{Window, WindowEventContext, WindowRequest},
};

type CallbackFn<State> = Box<dyn FnMut(&mut State, &mut CallbackContext<State>, Box<dyn Any>)>;
pub struct CallbackContext<'a, State> {
    pub windows: &'a mut HashMap<WindowId, Window>,
    pub add_callback: Box<dyn FnMut(CallbackFn<State>) -> CallbackId + 'a>,
    event_loop_proxy: EventLoopProxy<UserEvent>,
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
        let window = Window::new(window, root_widget);
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
        let address = address(id).ok_or(WidgetNotFound)?;
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
    pub event: Box<dyn Any + Send>,
}

#[derive(Debug, From)]
pub enum UserEvent {
    InvokeCallback(InvokeCallbackEvent),
    WindowRequest(WindowId, WindowRequest),
    ActionRequest(ActionRequestEvent),
}

scoped_thread_local!(pub static WINDOW_TARGET: EventLoopWindowTarget<UserEvent>);

fn dispatch_widget_callback<Event>(
    windows: &mut HashMap<WindowId, Window>,
    callback: &WidgetCallback<Event>,
    event: Event,
) {
    let Some(address) = address(callback.widget_id) else {
        return;
    };
    let Some(window) = windows.get_mut(&address.window_id) else {
        return;
    };
    let Some(root_widget) = window.root_widget.as_mut() else {
        return;
    };
    let Ok(widget) = get_widget_by_address_mut(root_widget.as_mut(), &address) else {
        return;
    };
    (callback.func)(widget, event);
}

pub fn run<State: 'static>(make_state: impl FnOnce(&mut CallbackContext<State>) -> State) {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event()
        .build()
        .expect("Event loop creation failed");

    let mut windows = HashMap::new();

    let shared_system_data = SharedSystemDataInner {
        address_book: HashMap::new(),
        font_system: FontSystem::new(),
        swash_cache: SwashCache::new(),
        font_metrics: cosmic_text::Metrics::new(24.0, 30.0),
        event_loop_proxy: event_loop.create_proxy(),
        palette: Palette {
            foreground: Color::BLACK,
            background: Color::WHITE,
            unfocused_input_border: Color::from_rgba8(200, 200, 200, 255),
            focused_input_border: Color::from_rgba8(100, 100, 255, 255),
            // foreground: Color::WHITE,
            // background: Color::BLACK,
        },
        timers: Timers::new(),
        clipboard: Clipboard::new().expect("failed to initialize clipboard"),
    };
    SYSTEM.with(|system| {
        *system.0.borrow_mut() = Some(shared_system_data);
    });

    let mut callback_maker = CallbackMaker::<State>::new();
    let mut callbacks = Callbacks::<State>::new();

    let event_loop_proxy = event_loop.create_proxy();

    let mut state = {
        let mut ctx = CallbackContext {
            windows: &mut windows,
            add_callback: Box::new(|f| callback_maker.add(f)),
            marker: PhantomData,
            event_loop_proxy: event_loop_proxy.clone(),
        };
        WINDOW_TARGET.set(&event_loop, || make_state(&mut ctx))
    };
    callbacks.add_all(&mut callback_maker);

    event_loop
        .run(move |event, window_target| {
            WINDOW_TARGET.set(window_target, || {
                while let Some(timer) = with_system(|system| system.timers.pop()) {
                    dispatch_widget_callback(&mut windows, &timer.callback, Instant::now());
                    // TODO: smarter redraw
                    for window in windows.values() {
                        window.inner.request_redraw();
                    }
                }

                let mut ctx = WindowEventContext {};

                match event {
                    Event::WindowEvent {
                        window_id,
                        event: ref wevent,
                    } => {
                        // TODO: smarter logic
                        if matches!(wevent, WindowEvent::CloseRequested) {
                            window_target.exit();
                        }
                        if let Some(window) = windows.get_mut(&window_id) {
                            window.handle_event(&mut ctx, event.map_nonuser_event().unwrap());
                        }
                    }

                    Event::UserEvent(event) => match event {
                        UserEvent::WindowRequest(window_id, request) => {
                            if let Some(window) = windows.get_mut(&window_id) {
                                window.handle_request(&mut ctx, request);
                            }
                        }
                        UserEvent::InvokeCallback(event) => {
                            {
                                let mut ctx = CallbackContext {
                                    windows: &mut windows,
                                    add_callback: Box::new(|f| callback_maker.add(f)),
                                    marker: PhantomData,
                                    event_loop_proxy: event_loop_proxy.clone(),
                                };

                                callbacks.call(&mut state, &mut ctx, event);
                            }
                            callbacks.add_all(&mut callback_maker);
                        }
                        UserEvent::ActionRequest(request) => {
                            println!("accesskit request: {:?}", request)
                        }
                    },
                    Event::AboutToWait => {
                        let next_timer = with_system(|system| system.timers.next_instant());
                        if let Some(next_timer) = next_timer {
                            // println!(
                            //     "wait until {:?} | {:?}",
                            //     next_timer,
                            //     next_timer - Instant::now()
                            // );
                            window_target.set_control_flow(ControlFlow::WaitUntil(next_timer));
                        } else {
                            // println!("wait forever");
                            window_target.set_control_flow(ControlFlow::Wait);
                        }
                    }
                    _ => {}
                }

                //if *control_flow == ControlFlow::Wait {
                //}
            });
        })
        .expect("Error while running event loop");
}
