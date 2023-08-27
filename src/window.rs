use std::{collections::HashSet, num::NonZeroU32};

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::Pixmap;
use winit::event::{ElementState, Event, ModifiersState, MouseButton, WindowEvent};

use crate::{
    draw::{DrawContext, Palette},
    event::{CursorMovedEvent, MouseInputEvent},
    types::{Point, Rect, Size},
    Widget,
};

pub struct Window {
    pub inner: winit::window::Window,
    pub softbuffer_context: softbuffer::Context,
    pub surface: softbuffer::Surface,
    pub cursor_position: Option<Point>,
    pub cursor_entered: bool,
    pub modifiers_state: ModifiersState,
    pressed_mouse_buttons: HashSet<MouseButton>,
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
            modifiers_state: ModifiersState::default(),
            pressed_mouse_buttons: HashSet::new(),
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

                // tiny-skia uses an RGBA format, while softbuffer uses XRGB. To convert, we need to
                // iterate over the pixels and shift the pixels over.
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
                WindowEvent::CursorMoved {
                    position,
                    device_id,
                    ..
                } => {
                    let pos = Point {
                        // TODO: is round() fine?
                        x: position.x.round() as i32,
                        y: position.y.round() as i32,
                    };
                    self.cursor_position = Some(pos);
                    if let Some(widget) = &mut self.widget {
                        widget.cursor_moved(&mut CursorMovedEvent {
                            device_id,
                            modifiers: self.modifiers_state,
                            pressed_mouse_buttons: &self.pressed_mouse_buttons,
                            pos,
                            font_system: ctx.font_system,
                            font_metrics: ctx.font_metrics,
                            palette: ctx.palette,
                        });
                        self.inner.request_redraw(); // TODO: smarter redraw
                    }
                }
                WindowEvent::ModifiersChanged(state) => {
                    self.modifiers_state = state;
                }
                WindowEvent::MouseInput {
                    device_id,
                    state,
                    button,
                    ..
                } => {
                    match state {
                        ElementState::Pressed => {
                            self.pressed_mouse_buttons.insert(button);
                        }
                        ElementState::Released => {
                            self.pressed_mouse_buttons.remove(&button);
                        }
                    }
                    if let Some(pos) = self.cursor_position {
                        if let Some(widget) = &mut self.widget {
                            widget.mouse_input(&mut MouseInputEvent {
                                device_id,
                                state,
                                button,
                                modifiers: self.modifiers_state,
                                pressed_mouse_buttons: &self.pressed_mouse_buttons,
                                pos,
                                font_system: ctx.font_system,
                                font_metrics: ctx.font_metrics,
                                palette: ctx.palette,
                            });
                            self.inner.request_redraw(); // TODO: smarter redraw
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

    pub fn set_widget<W: Widget + 'static>(&mut self, widget: Option<W>) {
        self.widget = widget.map(|w| Box::new(w) as _);
    }
}

pub struct WindowEventContext<'a> {
    pub font_system: &'a mut FontSystem,
    pub font_metrics: cosmic_text::Metrics,
    pub swash_cache: &'a mut SwashCache,
    pub palette: &'a mut Palette,
}
