use {
    anyhow::Context,
    std::{
        cmp::min,
        num::NonZeroU32,
        rc::Rc,
        time::{Duration, Instant},
    },
    winit::{
        application::ApplicationHandler,
        dpi::PhysicalSize,
        event::{StartCause, WindowEvent},
        event_loop::ActiveEventLoop,
        window::{Window, WindowAttributes},
    },
};

struct Handler {
    context: crate::Context,
    // Drop order must be maintained as
    // `surface` -> `softbuffer_context` -> `window`.
    surface: Option<softbuffer::Surface<Rc<winit::window::Window>, Rc<winit::window::Window>>>,
    softbuffer_context: Option<softbuffer::Context<Rc<winit::window::Window>>>,
    window: Option<Rc<Window>>,
    take_screenshot_at: Option<Instant>,
}

const WIDTH: u32 = 100;
const HEIGHT: u32 = 100;
const TITLE: &str = "__UITEST_DETECTOR";

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Rc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT))
                        .with_title(TITLE),
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
                let take_screenshot_at = self
                    .take_screenshot_at
                    .get_or_insert_with(|| Instant::now() + Duration::from_millis(100));
                event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                    *take_screenshot_at,
                ));
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

    fn new_events(&mut self, event_loop: &ActiveEventLoop, _cause: StartCause) {
        if self.take_screenshot_at.is_some_and(|t| t >= Instant::now()) {
            let windows = super::all_windows(&self.context).unwrap();
            let window = windows
                .iter()
                .find(|w| w.title().is_ok_and(|t| t == TITLE))
                .expect("detector window not found");
            let image = window.0.capture_image_without_unpaint().unwrap();
            println!("ok! {}x{}", image.width(), image.height());

            self.take_screenshot_at = None;
            event_loop.exit();
        }
    }
}

pub fn run(context: crate::Context) -> anyhow::Result<()> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut handler = Handler {
        context,
        surface: None,
        softbuffer_context: None,
        window: None,
        take_screenshot_at: None,
    };
    event_loop.run_app(&mut handler)?;
    Ok(())
}
