use {
    crate::IGNORED_PIXEL,
    anyhow::{anyhow, bail, ensure, Context as _},
    image::{Rgba, RgbaImage},
    objc2_application_services::{
        kAXTrustedCheckOptionPrompt, AXError, AXIsProcessTrustedWithOptions, AXUIElement, AXValue,
        AXValueType,
    },
    objc2_core_foundation::{
        CFArray, CFBoolean, CFDictionary, CFRetained, CFString, CFType, CGFloat, CGPoint, CGSize,
    },
    std::{
        ffi::c_void,
        hash::{DefaultHasher, Hash, Hasher},
        mem::MaybeUninit,
        process::Command,
        ptr::NonNull,
        sync::{Arc, Mutex},
    },
    tracing::{trace, warn},
};

// Offset between the window's outer and inner position.
// TODO: allow overriding it with an env var or determine it automatically.
const TITLE_OFFSET_Y: u32 = 28;

pub struct Context {}

impl Context {
    pub fn new() -> anyhow::Result<Self> {
        unsafe {
            let args =
                CFDictionary::from_slices(&[kAXTrustedCheckOptionPrompt], &[CFBoolean::new(true)]);
            let is_trusted = AXIsProcessTrustedWithOptions(Some(args.as_opaque()));
            if !is_trusted {
                bail!("process is not trusted");
            }
        }
        Ok(Self {})
    }

    // TODO: avoid iterating over all apps if only windows_by_pid is requested.
    pub fn all_windows(&self, context: &crate::Context) -> anyhow::Result<Vec<Window>> {
        unsafe {
            // We could get the list of apps using `NSWorkspace::sharedWorkspace().runningApplications()`.
            // However, this list never updates because we're not running a macos event loop
            // in this process. The easiest fix is to query the list of apps in a new process.
            let output = Command::new("osascript")
                .args([
                    "-e",
                    "tell application \"System Events\" to get the unix id of every process",
                ])
                .output()?;
            ensure!(output.status.success(), "osascript failed: {:?}", output);
            let output = String::from_utf8(output.stdout).context("invalid osascript output")?;
            let mut all_windows = Vec::new();
            for pid in output.trim_ascii().split(", ") {
                let pid = pid.parse::<i32>().context("pid is not a number")?;
                match get_app_windows(pid, context) {
                    Ok(windows) => all_windows.extend(windows),
                    Err(err) => {
                        trace!("failed to get app windows for pid {:?}: {:?}", pid, err);
                    }
                };
            }

            Ok(all_windows)
        }
    }

    pub fn active_window_id(&self) -> anyhow::Result<u32> {
        todo!("active_window_id")
    }
}

#[derive(Clone)]
pub struct Window {
    id: u32,
    pid: u32,
    ui_element: CFRetained<AXUIElement>,
    has_title: Arc<Mutex<Option<bool>>>,
    xcap_window: Arc<Mutex<Option<xcap::Window>>>,
    context: crate::Context,
}

impl Window {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// The window id
    pub fn id(&self) -> u32 {
        self.id
    }
    /// The window app name
    pub fn app_name(&self) -> anyhow::Result<String> {
        todo!("app_name")
    }
    /// The window title
    pub fn title(&self) -> anyhow::Result<String> {
        Ok(attribute(&self.ui_element, "AXTitle")?
            .context("no AXTitle attribute")?
            .downcast::<CFString>()
            .map_err(|_| anyhow!("title attribute is not string"))?
            .to_string())
    }

    fn has_title(&self) -> anyhow::Result<bool> {
        if let Some(v) = *self.has_title.lock().unwrap() {
            return Ok(v);
        }
        let window = self.xcap_window()?;
        let image = window.capture_image()?;
        // Heuristic: the window has a system frame if the corners are transparent.
        let has_title = image.get_pixel(0, 0).0[3] < 255;
        *self.has_title.lock().unwrap() = Some(has_title);
        Ok(has_title)
    }

    fn xcap_window(&self) -> anyhow::Result<xcap::Window> {
        if let Some(v) = &*self.xcap_window.lock().unwrap() {
            return Ok(v.clone());
        }
        // There is no way to convert between AXUIElement windows and Core Graphics
        // windows, so we have to try and find a window by position and size.
        let x = self.x()?;
        let y = self.y()?;
        let size_with_frame = self.outer_size()?;
        let mut matching_windows = Vec::new();
        for window in xcap::Window::all()? {
            if x == window.x()?
                && y == window.y()?
                && size_with_frame.width as u32 == window.width()?
                && size_with_frame.height as u32 == window.height()?
            {
                matching_windows.push(window);
            }
        }

        if matching_windows.len() == 1 {
            let window = matching_windows.remove(0);
            *self.xcap_window.lock().unwrap() = Some(window.clone());
            Ok(window)
        } else if matching_windows.is_empty() {
            bail!("no matching CG windows found");
        } else {
            for window in matching_windows {
                warn!(
                    "matching window: {:?} title={:?} app={:?}",
                    window.id(),
                    window.title(),
                    window.app_name()
                );
            }
            bail!("multiple matching CG windows found");
        }
    }

    fn position(&self) -> anyhow::Result<CGPoint> {
        let value = attribute(&self.ui_element, "AXPosition")?
            .context("missing position attribute")?
            .downcast::<AXValue>()
            .map_err(|_| anyhow!("position attribute is not AXValue"))?;
        get_cg_point(&value)
    }

    /// The window x coordinate.
    pub fn x(&self) -> anyhow::Result<i32> {
        Ok(self.position()?.x as i32)
    }

    /// The window x coordinate.
    pub fn y(&self) -> anyhow::Result<i32> {
        Ok(self.position()?.y as i32)
    }

    fn outer_size(&self) -> anyhow::Result<CGSize> {
        let value = attribute(&self.ui_element, "AXSize")?
            .context("missing position attribute")?
            .downcast::<AXValue>()
            .map_err(|_| anyhow!("position attribute is not AXValue"))?;
        get_cg_size(&value)
    }

    /// The window inner pixel width.
    pub fn width(&self) -> anyhow::Result<u32> {
        Ok(self.outer_size()?.width as u32)
    }

    /// The window inner pixel height.
    pub fn height(&self) -> anyhow::Result<u32> {
        let title_offset_y = if self.has_title()? { TITLE_OFFSET_Y } else { 0 };
        Ok((self.outer_size()?.height as u32).saturating_sub(title_offset_y))
    }

    pub fn is_minimized(&self) -> anyhow::Result<bool> {
        todo!("is_minimized")
    }

    pub fn is_maximized(&self) -> anyhow::Result<bool> {
        todo!("is_maximized")
    }

    pub fn capture_image(&self) -> anyhow::Result<RgbaImage> {
        let window = self.xcap_window()?;
        let image = window.capture_image()?;
        unpaint_window_frame(image)
    }

    pub fn activate(&self) -> anyhow::Result<()> {
        todo!("activate")
    }

    /// Move the mouse pointer to the coordinates specified relative to the window's inner position.
    pub fn mouse_move(&self, x: i32, y: i32) -> anyhow::Result<()> {
        let title_offset_y = if self.has_title()? { TITLE_OFFSET_Y } else { 0 };
        let position = self.position()?;
        self.context.mouse_move_global(
            x + position.x as i32,
            y + position.y as i32 + title_offset_y as i32,
        )
    }

    pub fn minimize(&self) -> anyhow::Result<()> {
        todo!()
    }

    pub fn close(&self) -> anyhow::Result<()> {
        unsafe {
            if let Ok(Some(close_button)) = attribute(&self.ui_element, "AXCloseButton") {
                let r = close_button
                    .downcast::<AXUIElement>()
                    .map_err(|_| anyhow!("AXCloseButton attribute is not AXUIElement"))?
                    .perform_action(&CFString::from_static_str("AXPress"));
                if r != AXError::Success {
                    bail!("failed to perform AXPress: {}", ax_error_text(r));
                }
            } else {
                // Some windows support direct "Close" action
                let r = self
                    .ui_element
                    .perform_action(&CFString::from_static_str("AXClose"));
                if r != AXError::Success {
                    bail!("failed to perform AXClose: {}", ax_error_text(r));
                }
            }
        }
        Ok(())
    }

    /// Change the window's inner size to the specified values.
    pub fn resize(&self, width: i32, height: i32) -> anyhow::Result<()> {
        let title_offset_y = if self.has_title()? { TITLE_OFFSET_Y } else { 0 };
        unsafe {
            let mut new_size = CGSize {
                width: width as CGFloat,
                height: (height + title_offset_y as i32) as CGFloat,
            };
            let value = AXValue::new(
                AXValueType::CGSize,
                NonNull::new(&mut new_size as *mut CGSize as *mut c_void)
                    .expect("null pointer to stack value"),
            )
            .context("AXValue::new failed")?;
            let r = self
                .ui_element
                .set_attribute_value(&CFString::from_static_str("AXSize"), &value);
            if r != AXError::Success {
                bail!("failed to set AXSize: {}", ax_error_text(r));
            }
        }
        Ok(())
    }
}

unsafe fn get_app_windows(pid: i32, context: &crate::Context) -> anyhow::Result<Vec<Window>> {
    unsafe {
        let mut outputs = Vec::new();
        let app = AXUIElement::new_application(pid);
        let Some(windows) = attribute(&app, "AXWindows")? else {
            return Ok(Vec::new());
        };
        let windows = windows
            .downcast::<CFArray>()
            .map_err(|_| anyhow!("windows attribute is not array"))?;

        let num_windows = windows.len();
        for i in 0..num_windows {
            let window = CFRetained::<AXUIElement>::from_raw(
                NonNull::new(windows.value_at_index(i as isize) as *mut _).unwrap(),
            );
            outputs.push(Window {
                // New windows are added to the front of the list, so
                // `num_windows - i` is better than `i`.
                id: window_id(pid, num_windows - i),
                pid: pid as u32,
                ui_element: window,
                context: context.clone(),
                has_title: Arc::new(Mutex::new(None)),
                xcap_window: Arc::new(Mutex::new(None)),
            });
        }
        Ok(outputs)
    }
}

fn get_cg_point(value: &AXValue) -> anyhow::Result<CGPoint> {
    unsafe {
        let mut output = MaybeUninit::<CGPoint>::uninit();
        let r = value.value(
            AXValueType::CGPoint,
            NonNull::new(output.as_mut_ptr() as *mut c_void)
                .context("stack variable with 0 address")?,
        );
        if !r {
            bail!("value is not CGPoint");
        }
        Ok(output.assume_init())
    }
}

fn get_cg_size(value: &AXValue) -> anyhow::Result<CGSize> {
    unsafe {
        let mut output = MaybeUninit::<CGSize>::uninit();
        let r = value.value(
            AXValueType::CGSize,
            NonNull::new(output.as_mut_ptr() as *mut c_void)
                .context("stack variable with 0 address")?,
        );
        if !r {
            bail!("value is not CGSize");
        }
        Ok(output.assume_init())
    }
}

fn attribute(
    element: &AXUIElement,
    name: &'static str,
) -> anyhow::Result<Option<CFRetained<CFType>>> {
    unsafe {
        let mut output = MaybeUninit::<*const CFType>::uninit();
        let r = element.copy_attribute_value(
            &CFString::from_static_str(name),
            NonNull::new(output.as_mut_ptr()).context("stack variable with 0 address")?,
        );
        if r != AXError::Success {
            if r == AXError::NoValue || r == AXError::CannotComplete || r == AXError::APIDisabled {
                return Ok(None);
            }
            bail!("failed to get attribute {}: {}", name, ax_error_text(r));
        }
        Ok(Some(CFRetained::<CFType>::retain(
            NonNull::new(output.assume_init() as *mut CFType)
                .context("stack variable with 0 address")?,
        )))
    }
}

#[allow(dead_code)]
fn action_names(element: &AXUIElement) -> anyhow::Result<Vec<String>> {
    unsafe {
        let mut output = MaybeUninit::<*const CFArray>::uninit();
        let r = element.copy_action_names(
            NonNull::new(output.as_mut_ptr()).context("stack variable with 0 address")?,
        );
        if r != AXError::Success {
            if r == AXError::NoValue || r == AXError::CannotComplete || r == AXError::APIDisabled {
                return Ok(Vec::new());
            }
            bail!("failed to get action names: {}", ax_error_text(r));
        }
        let array = CFRetained::<CFArray>::retain(
            NonNull::new(output.assume_init() as *mut CFArray)
                .context("stack variable with 0 address")?,
        );

        let mut output = Vec::new();
        let count = array.len();
        for i in 0..count {
            let name = CFRetained::<CFString>::from_raw(
                NonNull::new(array.value_at_index(i as isize) as *mut _).unwrap(),
            );
            output.push(name.to_string());
        }
        Ok(output)
    }
}

#[allow(dead_code)]
fn attribute_names(element: &AXUIElement) -> anyhow::Result<Vec<String>> {
    unsafe {
        let mut output = MaybeUninit::<*const CFArray>::uninit();
        let r = element.copy_attribute_names(
            NonNull::new(output.as_mut_ptr()).context("stack variable with 0 address")?,
        );
        if r != AXError::Success {
            if r == AXError::NoValue || r == AXError::CannotComplete || r == AXError::APIDisabled {
                return Ok(Vec::new());
            }
            bail!("failed to get attribute names: {}", ax_error_text(r));
        }
        let array = CFRetained::<CFArray>::retain(
            NonNull::new(output.assume_init() as *mut CFArray)
                .context("stack variable with 0 address")?,
        );

        let mut output = Vec::new();
        let count = array.len();
        for i in 0..count {
            let name = CFRetained::<CFString>::from_raw(
                NonNull::new(array.value_at_index(i as isize) as *mut _).unwrap(),
            );
            output.push(name.to_string());
        }
        Ok(output)
    }
}

// MacOS Accessibility API doesn't expose any IDs.
fn window_id(window_pid: i32, window_index: usize) -> u32 {
    let mut s = DefaultHasher::new();
    (window_pid, window_index).hash(&mut s);
    s.finish() as u32
}

fn ax_error_text(error: AXError) -> String {
    match error {
        AXError::APIDisabled => "APIDisabled".into(),
        AXError::ActionUnsupported => "ActionUnsupported".into(),
        AXError::AttributeUnsupported => "AttributeUnsupported".into(),
        AXError::CannotComplete => "CannotComplete".into(),
        AXError::Failure => "Failure".into(),
        AXError::IllegalArgument => "IllegalArgument".into(),
        AXError::InvalidUIElement => "InvalidUIElement".into(),
        AXError::InvalidUIElementObserver => "InvalidUIElementObserver".into(),
        AXError::NoValue => "NoValue".into(),
        AXError::NotEnoughPrecision => "NotEnoughPrecision".into(),
        AXError::NotImplemented => "NotImplemented".into(),
        AXError::NotificationAlreadyRegistered => "NotificationAlreadyRegistered".into(),
        AXError::NotificationNotRegistered => "NotificationNotRegistered".into(),
        AXError::NotificationUnsupported => "NotificationUnsupported".into(),
        AXError::ParameterizedAttributeUnsupported => "ParameterizedAttributeUnsupported".into(),
        AXError::Success => "Success".into(),
        _ => format!("unknown error {}", error.0),
    }
}

// Window screenshots contain a system window frame, but we only need the content.
fn unpaint_window_frame(mut image: RgbaImage) -> anyhow::Result<RgbaImage> {
    let background = Rgba([255, 255, 255, 255]);
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

    // Ignore semi-transparent border pixels.
    for x in [0, width - 1] {
        for y in 0..height {
            image.put_pixel(x, y, IGNORED_PIXEL);
        }
    }

    // Remove window title.
    let new_height = height - TITLE_OFFSET_Y;
    let mut new_image = RgbaImage::from_pixel(width, new_height, background);
    let stride = new_image.sample_layout().height_stride;
    (*new_image).copy_from_slice(&(*image)[TITLE_OFFSET_Y as usize * stride..]);

    // Ignore title shadow and semi-transparent border pixels.
    for y in [0, new_height - 1] {
        for x in 0..width {
            new_image.put_pixel(x, y, IGNORED_PIXEL);
        }
    }

    Ok(new_image)
}
