use std::num::NonZeroU32;

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{Color, Pixmap};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    draw::{DrawContext, Palette},
    WidgetContainer,
};

pub fn run(mut root_widget: WidgetContainer) {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("My window title")
        .build(&event_loop)
        .unwrap();

    let context = unsafe { softbuffer::Context::new(&window) }.unwrap();
    let mut surface = unsafe { softbuffer::Surface::new(&context, &window) }.unwrap();

    // A FontSystem provides access to detected system fonts, create one per application
    let mut font_system = FontSystem::new();

    // A SwashCache stores rasterized glyphs, create one per application
    let mut swash_cache = SwashCache::new();

    // Text metrics indicate the font size and line height of a buffer
    let font_metrics = cosmic_text::Metrics::new(100.0, 150.0);

    let mut palette = Palette {
        foreground: Color::BLACK,
        background: Color::WHITE,
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                // Grab the window's client area dimensions
                let (width, height) = {
                    let size = window.inner_size();
                    (size.width, size.height)
                };

                // Resize surface if needed
                surface
                    .resize(
                        NonZeroU32::new(width).unwrap(),
                        NonZeroU32::new(height).unwrap(),
                    )
                    .unwrap();

                // Draw something in the window
                let mut buffer = surface.buffer_mut().unwrap();

                let mut pixmap = Pixmap::new(width, height).unwrap();
                let mut ctx = DrawContext {
                    self_info: &mut root_widget.info,
                    pixmap: &mut pixmap,
                    font_system: &mut font_system,
                    font_metrics,
                    swash_cache: &mut swash_cache,
                    palette: &mut palette,
                };
                // TODO: widget should fill instead?
                ctx.pixmap.fill(ctx.palette.background);
                root_widget.widget.draw(&mut ctx);

                buffer.copy_from_slice(bytemuck::cast_slice(pixmap.data()));

                buffer.iter_mut().for_each(|pixel| {
                    let [r, g, b, _] = pixel.to_ne_bytes();
                    *pixel = (b as u32) | ((g as u32) << 8) | ((r as u32) << 16);
                });

                //redraw(&mut buffer, width as usize, height as usize, flag);
                buffer.present().unwrap();
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { .. },
                window_id,
            } if window_id == window.id() => {
                window.request_redraw();
            }

            _ => {}
        }
    });
}
