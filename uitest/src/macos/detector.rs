use {
    std::{cmp::min, num::NonZeroU32, rc::Rc},
    winit::{
        application::ApplicationHandler,
        dpi::PhysicalSize,
        event::WindowEvent,
        event_loop::ActiveEventLoop,
        window::{Window, WindowAttributes},
    },
};

struct Handler {
    // Drop order must be maintained as
    // `surface` -> `softbuffer_context` -> `window`.
    surface: Option<softbuffer::Surface<Rc<winit::window::Window>, Rc<winit::window::Window>>>,
    softbuffer_context: Option<softbuffer::Context<Rc<winit::window::Window>>>,
    window: Option<Rc<Window>>,
}

const WIDTH: u32 = 100;
const HEIGHT: u32 = 100;

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Rc::new(
            event_loop
                .create_window(
                    WindowAttributes::default().with_inner_size(PhysicalSize::new(WIDTH, HEIGHT)),
                )
                .unwrap(),
        );
        let softbuffer_context = softbuffer::Context::new(window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&softbuffer_context, window.clone()).unwrap();
        self.window = Some(window);
        self.softbuffer_context = Some(softbuffer_context);
        self.surface = Some(surface);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                let size = self.window.as_ref().unwrap().inner_size();
                self.surface
                    .as_mut()
                    .unwrap()
                    .resize(
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    )
                    .unwrap();
                let mut buffer = self.surface.as_mut().unwrap().buffer_mut().unwrap();
                for y in 0..min(HEIGHT, size.height) {
                    for x in 0..min(WIDTH, size.width) {
                        buffer[(y * size.width + x) as usize] =
                            ((x * 2) << 16) | ((y * 2) << 8) | 255;
                    }
                }

                buffer.present().unwrap();
            }
            WindowEvent::CloseRequested => {
                self.surface = None;
                self.softbuffer_context = None;
                self.window = None;
                event_loop.exit();
            }
            _ => {}
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut handler = Handler {
        surface: None,
        softbuffer_context: None,
        window: None,
    };
    event_loop.run_app(&mut handler)?;
    Ok(())
}
