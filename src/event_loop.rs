use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::Color;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    draw::Palette,
    window::{Window, WindowEventContext},
    Widget,
};

pub fn run(root_widget: impl Widget + 'static) {
    let event_loop = EventLoop::new();

    let mut window = Window::new(
        WindowBuilder::new()
            .with_title("My window title")
            .build(&event_loop)
            .unwrap(),
        Some(Box::new(root_widget)),
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

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        let mut ctx = WindowEventContext {
            font_system: &mut font_system,
            font_metrics,
            swash_cache: &mut swash_cache,
            palette: &mut palette,
        };

        match &event {
            Event::RedrawRequested(window_id) if *window_id == window.inner.id() => {
                window.handle_event(&mut ctx, event);
            }
            Event::WindowEvent {
                window_id,
                event: wevent,
            } if *window_id == window.inner.id() => {
                if matches!(wevent, WindowEvent::CloseRequested) {
                    *control_flow = ControlFlow::Exit;
                }
                window.handle_event(&mut ctx, event);
            }
            _ => {}
        }
    });
}
