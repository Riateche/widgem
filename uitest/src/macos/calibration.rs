use {
    crate::{Context, IGNORED_PIXEL},
    anyhow::{bail, ensure, Context as _},
    cadd::{
        ops::{Cadd, CsignedDiff, Csub},
        prelude::{Cinto, IntoType},
    },
    image::{imageops::crop_imm, RgbaImage},
    std::{
        cmp::min,
        collections::HashMap,
        num::NonZeroU32,
        rc::Rc,
        time::{Duration, Instant},
    },
    tracing::{error, info, trace},
    winit::{
        application::ApplicationHandler,
        dpi::PhysicalSize,
        event::{StartCause, WindowEvent},
        event_loop::ActiveEventLoop,
        window::{Window, WindowAttributes},
    },
};

struct CalibrationApp {
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

impl ApplicationHandler for CalibrationApp {
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
            let image = window.0.capture_image_without_calibration().unwrap();
            match analyze(image) {
                Ok(info) => {
                    info!(?info, "macos screenshot calibration success");
                    *self.context.0.imp.calibration.lock().unwrap() = Some(info);
                }
                Err(error) => error!(?error, "failed to calibrate macos screenshot"),
            }

            self.take_screenshot_at = None;
            event_loop.exit();
        }
    }
}

pub fn run(context: crate::Context) -> anyhow::Result<()> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut handler = CalibrationApp {
        context,
        surface: None,
        softbuffer_context: None,
        window: None,
        take_screenshot_at: None,
    };
    event_loop.run_app(&mut handler)?;
    Ok(())
}

fn analyze(image: RgbaImage) -> anyhow::Result<CalibrationInfo> {
    //image.save("/tmp/1.png").unwrap();
    let x = analyze_axis(&image, Axis::X)?;
    let y = analyze_axis(&image, Axis::Y)?;
    Ok(CalibrationInfo { x, y })
}

#[derive(Debug)]
enum Axis {
    X,
    Y,
}

#[derive(Debug, Clone)]
pub struct CalibrationInfo {
    x: AxisCalibrationInfo,
    y: AxisCalibrationInfo,
}

#[derive(Debug, Clone)]
struct AxisCalibrationInfo {
    skip: u32,
    ignore_start: u32,
    ignore_end: u32,
}

fn analyze_axis(image: &RgbaImage, axis: Axis) -> anyhow::Result<AxisCalibrationInfo> {
    // "line" runs along `axis`
    let mut line_index_diffs = Vec::new();
    let num_lines = match axis {
        Axis::X => image.width(),
        Axis::Y => image.height(),
    };
    let line_length = match axis {
        Axis::X => image.height(),
        Axis::Y => image.width(),
    };
    let expected_num_lines = match axis {
        Axis::X => WIDTH as f32,
        Axis::Y => HEIGHT as f32,
    };
    let expected_line_length = match axis {
        Axis::X => HEIGHT as f32,
        Axis::Y => WIDTH as f32,
    };
    for line_index in 0..num_lines {
        // reds for X, green for Y
        let mut reds_or_greens = HashMap::<u8, usize>::new();
        let mut blues = HashMap::<u8, usize>::new();
        for pixel_index in 0..line_length {
            let pixel = match axis {
                Axis::X => image.get_pixel(line_index, pixel_index),
                Axis::Y => image.get_pixel(pixel_index, line_index),
            };
            let red = pixel.0[0];
            let green = pixel.0[1];
            let blue = pixel.0[2];
            let red_or_green = match axis {
                Axis::X => red,
                Axis::Y => green,
            };
            reds_or_greens
                .entry(red_or_green)
                .or_default()
                .cadd_assign(1)?;
            blues.entry(blue).or_default().cadd_assign(1)?;
        }
        // or red
        let (&green_value, &green_count) = reds_or_greens
            .iter()
            .max_by_key(|v| v.1)
            .context("no pixels found")?;
        let (&blue_value, &blue_count) = blues
            .iter()
            .max_by_key(|v| v.1)
            .context("no pixels found")?;
        if blue_value.into_type::<u32>() == BLUE_VALUE
            && green_count as f32 >= expected_line_length * THRESHOLD
            && blue_count as f32 >= expected_line_length * THRESHOLD
        {
            let real_line_index = green_value as u32 / RG_SCALE;
            let line_index_diff = line_index.csigned_diff(real_line_index)?;
            trace!("line_index={line_index} line_index_diff={line_index_diff}");
            line_index_diffs.push(Some(line_index_diff));
        } else {
            line_index_diffs.push(None);
            trace!("line_index={line_index} undetected");
        }
        trace!("line_index={line_index} greens={reds_or_greens:?} blues={blues:?}",);
    }
    let skip = line_index_diffs
        .iter()
        .copied()
        .flatten()
        .max()
        .context("no pixels found")?;
    if skip < 0 {
        bail!("unexpected negative skip={skip}");
    }
    let matching_lines = line_index_diffs.iter().filter(|v| **v == Some(skip));
    if (matching_lines.count() as f32) < expected_num_lines * THRESHOLD {
        bail!("no consistent line_index_diff found for {axis:?}");
    }
    let skip_usize: usize = skip.cinto()?;

    let remaining_diffs = line_index_diffs
        .get(skip_usize..)
        .context("y_diff out of bounds")?;

    let half_len = remaining_diffs.len() / 2;

    let ignore_start = half_len.csub(
        remaining_diffs[..half_len]
            .iter()
            .rev()
            .copied()
            .filter(|v| *v == Some(skip))
            .count(),
    )?;

    let ignore_end = half_len.csub(
        remaining_diffs[remaining_diffs.len() - half_len..]
            .iter()
            .copied()
            .filter(|v| *v == Some(skip))
            .count(),
    )?;

    Ok(AxisCalibrationInfo {
        skip: skip.cinto()?,
        ignore_start: ignore_start.cinto()?,
        ignore_end: ignore_end.cinto()?,
    })
}

// Window screenshots on MacOS contain a system window frame, but we only need the content.
pub fn adjust_image(ctx: &Context, mut image: RgbaImage) -> anyhow::Result<RgbaImage> {
    let width = image.width();
    let height = image.height();

    // Heuristic: the window has a system frame if the corners are transparent.
    if image.get_pixel(0, 0).0[3] == 255 {
        return Ok(image);
    }

    ensure!(width > 0 && height > 0);

    // Ignore rounded corners at the bottom.
    for x in 0..width {
        for y in (0..height).rev() {
            const CORNER_RADIUS_SQ: u32 = 13 * 13;
            let dist_sq1 = x * x + (height - y) * (height - y);
            let dist_sq2 = (width - x) * (width - x) + (height - y) * (height - y);
            if dist_sq1 < CORNER_RADIUS_SQ || dist_sq2 < CORNER_RADIUS_SQ {
                image.put_pixel(x, y, IGNORED_PIXEL);
            }
        }
    }

    let Some(info) = ctx.0.imp.calibration.lock().unwrap().clone() else {
        return Ok(image);
    };

    if info.x.skip > 0 || info.y.skip > 0 {
        image = crop_imm(
            &image,
            info.x.skip,
            info.y.skip,
            image.width() - info.x.skip,
            image.height() - info.y.skip,
        )
        .to_image();
    }

    let width = image.width();
    let height = image.height();

    for x in 0..info.x.ignore_start {
        for y in 0..height {
            image.put_pixel(x, y, IGNORED_PIXEL);
        }
    }

    for x in (width - info.x.ignore_end)..width {
        for y in 0..height {
            image.put_pixel(x, y, IGNORED_PIXEL);
        }
    }

    for y in 0..info.y.ignore_start {
        for x in 0..width {
            image.put_pixel(x, y, IGNORED_PIXEL);
        }
    }

    for y in (height - info.y.ignore_end)..height {
        for x in 0..width {
            image.put_pixel(x, y, IGNORED_PIXEL);
        }
    }

    Ok(image)
}
