use std::{fmt::Debug, marker::PhantomData};

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::Color;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};

use crate::{
    callback::Callback,
    draw::Palette,
    window::{Window, WindowEventContext},
};

pub struct CallbackContext<'a, State> {
    pub window: &'a mut Window,
    //...
    marker: PhantomData<State>,
}

impl<'a, State> CallbackContext<'a, State> {
    pub fn callback<Event>(
        &self,
        func: impl Fn(&mut State, &mut CallbackContext<'_, State>, Event),
    ) -> Callback<State, Event> {
        todo!()
    }
}

pub struct InvokeCallbackEvent<State>(pub Box<dyn FnOnce(&mut State, &mut CallbackContext<State>)>);

pub enum UserEvent<State> {
    InvokeCallback(InvokeCallbackEvent<State>),
}

impl<State> Debug for UserEvent<State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvokeCallback(arg0) => f.debug_tuple("InvokeCallback").finish(),
        }
    }
}

pub fn run<State: 'static>(make_state: impl FnOnce(&mut CallbackContext<State>) -> State) {
    let event_loop = EventLoopBuilder::<UserEvent<State>>::with_user_event().build();

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
    };

    let mut ctx = CallbackContext {
        window: &mut window,
        marker: PhantomData,
    };
    let mut state = make_state(&mut ctx);

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
                    let mut ctx = CallbackContext {
                        window: &mut window,
                        marker: PhantomData,
                    };
                    (event.0)(&mut state, &mut ctx);
                }
            },
            _ => {}
        }
    });
}
