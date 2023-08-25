use std::num::NonZeroU32;

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::Pixmap;
use winit::event::{Event, WindowEvent};

use crate::{
    draw::{DrawContext, Palette},
    event::MouseInputEvent,
    types::{Point, Rect, Size},
    Widget,
};

pub struct Window {
    pub inner: winit::window::Window,
    pub softbuffer_context: softbuffer::Context,
    pub surface: softbuffer::Surface,
    pub cursor_position: Option<Point>,
    pub cursor_entered: bool,
    pub widget: Option<Box<dyn Widget>>,
}

impl Window {
    pub fn new(inner: winit::window::Window, widget: Option<Box<dyn Widget>>) -> Self {
        let softbuffer_context = unsafe { softbuffer::Context::new(&inner) }.unwrap();
        Self {
            surface: unsafe { softbuffer::Surface::new(&softbuffer_context, &inner) }.unwrap(),
            softbuffer_context,
            inner,
            cursor_position: None,
            cursor_entered: false,
            widget,
        }
    }

    pub fn handle_event(&mut self, ctx: &mut WindowEventContext<'_>, event: Event<()>) {
        match event {
            Event::RedrawRequested(_) => {
                // Grab the window's client area dimensions
                let (width, height) = {
                    let size = self.inner.inner_size();
                    (size.width, size.height)
                };

                // Resize surface if needed
                self.surface
                    .resize(
                        NonZeroU32::new(width).unwrap(),
                        NonZeroU32::new(height).unwrap(),
                    )
                    .unwrap();

                // Draw something in the window
                let mut buffer = self.surface.buffer_mut().unwrap();

                let mut pixmap = Pixmap::new(width, height).unwrap();
                let mut ctx = DrawContext {
                    rect: Rect {
                        top_left: Point::default(),
                        size: Size {
                            x: width as i32,
                            y: height as i32,
                        },
                    },
                    pixmap: &mut pixmap,
                    font_system: ctx.font_system,
                    font_metrics: ctx.font_metrics,
                    swash_cache: ctx.swash_cache,
                    palette: ctx.palette,
                };
                // TODO: widget should fill instead?
                ctx.pixmap.fill(ctx.palette.background);
                if let Some(widget) = &mut self.widget {
                    widget.draw(&mut ctx);
                }

                buffer.copy_from_slice(bytemuck::cast_slice(pixmap.data()));

                buffer.iter_mut().for_each(|pixel| {
                    let [r, g, b, _] = pixel.to_ne_bytes();
                    *pixel = (b as u32) | ((g as u32) << 8) | ((r as u32) << 16);
                });

                //redraw(&mut buffer, width as usize, height as usize, flag);
                buffer.present().unwrap();
            }
            Event::WindowEvent { event, .. } => match event {
                // TODO: should use device id?
                WindowEvent::CursorEntered { .. } => {
                    self.cursor_entered = true;
                }
                WindowEvent::CursorLeft { .. } => {
                    self.cursor_entered = false;
                    self.cursor_position = None;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pos = Point {
                        // TODO: is round() fine?
                        x: position.x.round() as i32,
                        y: position.y.round() as i32,
                    };
                    self.cursor_position = Some(pos);
                }
                WindowEvent::MouseInput {
                    device_id,
                    state,
                    button,
                    modifiers,
                } => {
                    if let Some(pos) = self.cursor_position {
                        if let Some(widget) = &mut self.widget {
                            widget.mouse_input(&mut MouseInputEvent {
                                device_id,
                                state,
                                button,
                                modifiers,
                                pos,
                                font_metrics: ctx.font_metrics,
                                palette: ctx.palette,
                            });
                        }
                    } else {
                        println!("warning: no cursor position in mouse input handler");
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

pub struct WindowEventContext<'a> {
    pub font_system: &'a mut FontSystem,
    pub font_metrics: cosmic_text::Metrics,
    pub swash_cache: &'a mut SwashCache,
    pub palette: &'a mut Palette,
}
