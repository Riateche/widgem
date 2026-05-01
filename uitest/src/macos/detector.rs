use {
    image::RgbaImage,
    std::{
        cmp::min,
        collections::{BTreeMap, HashMap},
        convert::identity,
        num::NonZeroU32,
        rc::Rc,
        time::{Duration, Instant},
    },
    tracing::trace,
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

const RG_SCALE: u32 = 2;
const BLUE_VALUE: u32 = 252;
const THRESHOLD: f32 = 0.5;

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
                            ((x * RG_SCALE) << 16) | ((y * RG_SCALE) << 8) | BLUE_VALUE;
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
            analyze(image);

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

fn analyze(image: RgbaImage) {
    image.save("/tmp/1.png").unwrap();
    let mut y_diffs = Vec::new();
    for y in 0..image.height() {
        let mut greens = HashMap::<u8, usize>::new();
        let mut blues = HashMap::<u8, usize>::new();
        for x in 0..image.width() {
            let pixel = image.get_pixel(x, y);
            //let red = pixel.0[1];
            let green = pixel.0[1];
            let blue = pixel.0[2];
            *greens.entry(green).or_default() += 1;
            *blues.entry(blue).or_default() += 1;
        }
        let (green_value, green_count) = greens.iter().max_by_key(|v| v.1).unwrap();
        let (blue_value, blue_count) = blues.iter().max_by_key(|v| v.1).unwrap();
        if *blue_value as u32 == BLUE_VALUE
            && *green_count as f32 >= WIDTH as f32 * THRESHOLD
            && *blue_count as f32 >= WIDTH as f32 * THRESHOLD
        {
            let real_y = *green_value as u32 / RG_SCALE;
            let y_diff = y - real_y;
            println!("y={y} y_diff={y_diff}");
            y_diffs.push(Some(y_diff));
        } else {
            y_diffs.push(None);
            println!("y={y} undetected");
        }
        trace!("y={} reds={:?} blues={:?}", y, greens, blues);
    }
    let y_diff = y_diffs.iter().copied().flatten().max().unwrap();
    if (y_diffs.iter().filter(|v| **v == Some(y_diff)).count() as f32) < HEIGHT as f32 * THRESHOLD {
        panic!("no consistent y_diff found");
    }

    std::process::exit(22);
}
