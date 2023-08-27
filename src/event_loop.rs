use std::{any::Any, fmt::Debug, marker::PhantomData};

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::Color;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::WindowBuilder,
};

use crate::{
    callback::{Callback, CallbackId, CallbackMaker, Callbacks},
    draw::Palette,
    window::{Window, WindowEventContext},
};

type CallbackFn<State> = Box<dyn FnMut(&mut State, &mut CallbackContext<State>, Box<dyn Any>)>;
pub struct CallbackContext<'a, State> {
    pub window: &'a mut Window,
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
            window: self.window,
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

pub fn run<State: 'static>(make_state: impl FnOnce(&mut CallbackContext<State>) -> State) {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    let mut window = Window::new(
        WindowBuilder::new()
            .with_title("My window title")
            .build(&event_loop)
            .unwrap(),
        None,
    );

    // A FontSystem provides access to detected system fonts, create one per application
    let mut font_system = FontSystem::new();

    // A SwashCache stores rasterized glyphs, create one per application
    let mut swash_cache = SwashCache::new();

    // Text metrics indicate the font size and line height of a buffer
    let font_metrics = cosmic_text::Metrics::new(24.0, 30.0);

    let mut palette = Palette {
        foreground: Color::BLACK,
        background: Color::WHITE,
        // foreground: Color::WHITE,
        // background: Color::BLACK,
    };

    let mut callback_maker = CallbackMaker::<State>::new();
    let mut callbacks = Callbacks::<State>::new();

    let event_loop_proxy = event_loop.create_proxy();

    let mut state = {
        let mut ctx = CallbackContext {
            window: &mut window,
            add_callback: Box::new(|f| callback_maker.add(f)),
            marker: PhantomData,
            event_loop_proxy: event_loop_proxy.clone(),
        };
        make_state(&mut ctx)
    };
    callbacks.add_all(&mut callback_maker);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        let mut ctx = WindowEventContext {
            font_system: &mut font_system,
            font_metrics,
            swash_cache: &mut swash_cache,
            palette: &mut palette,
        };

        match event {
            Event::RedrawRequested(window_id) if window_id == window.inner.id() => {
                window.handle_event(&mut ctx, event.map_nonuser_event().unwrap());
            }
            Event::WindowEvent {
                window_id,
                event: ref wevent,
            } if window_id == window.inner.id() => {
                if matches!(wevent, WindowEvent::CloseRequested) {
                    *control_flow = ControlFlow::Exit;
                }
                window.handle_event(&mut ctx, event.map_nonuser_event().unwrap());
            }
            Event::UserEvent(event) => match event {
                UserEvent::InvokeCallback(event) => {
                    {
                        let mut ctx = CallbackContext {
                            window: &mut window,
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
}
