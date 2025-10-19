use {
    crate::{discover_snapshots, SingleSnapshotFiles, Window},
    anyhow::{bail, Context as _},
    chrono::Utc,
    fs_err::create_dir_all,
    image::{ImageReader, RgbaImage},
    itertools::Itertools,
    std::{
        collections::BTreeMap,
        ffi::OsString,
        fmt::Display,
        mem,
        path::{Path, PathBuf},
        process::{self, Child, Command},
        sync::{Arc, Mutex},
        thread::sleep,
        time::{Duration, Instant},
    },
    uitest::IGNORED_PIXEL,
    widgem::{widgets::RootWidget, AppBuilder},
};

/// Interval between capture attempts for a changed or blinking snapshot.
const CAPTURE_INTERVAL: Duration = Duration::from_millis(30);
/// Maximum time for waiting for a changed or blinking snapshot.
const MAX_CAPTURE_DURATION: Duration = Duration::from_secs(2);
/// Time required for a window snapshots to be unchanged before the snapshot is accepted.
const STATIONARY_INTERVAL: Duration = Duration::from_millis(200);
/// Delay before capturing a snapshot without change detection heuristics.
const SIMPLE_CAPTURE_DELAY: Duration = Duration::from_millis(500);
/// Interval between attempts to find windows by criteria.
const WAIT_FOR_WINDOWS_INTERVAL: Duration = Duration::from_millis(200);
/// Maximum time for finding windows by criteria.
const WAIT_FOR_WINDOWS_DURATION: Duration = Duration::from_secs(15);

pub(crate) struct CheckContext {
    uitest_context: uitest::Context,
    test_name: String,
    test_case_dir: PathBuf,
    last_snapshot_index: u32,
    exe_path: OsString,
    pid: Option<u32>,
    child: Option<Child>,
    unverified_files: BTreeMap<u32, SingleSnapshotFiles>,
    fails: Vec<String>,
    blinking_expected: bool,
    changing_expected: bool,
    last_snapshots: BTreeMap<u32, RgbaImage>,
}

impl Drop for CheckContext {
    fn drop(&mut self) {
        if let Some(mut child) = self.take_child() {
            println!(
                "killing child with pid={} after abnormal test exit",
                child.id(),
            );
            if let Err(err) = child.kill() {
                println!("error while killing child: {err:?}");
            }
        }
    }
}

impl CheckContext {
    fn new(
        uitest_context: uitest::Context,
        test_name: String,
        test_case_dir: PathBuf,
        exe_path: OsString,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            unverified_files: discover_snapshots(&test_case_dir)?,
            uitest_context,
            test_name,
            test_case_dir,
            exe_path,
            pid: None,
            child: None,
            last_snapshot_index: 0,
            last_snapshots: BTreeMap::new(),
            fails: Vec::new(),
            blinking_expected: false,
            changing_expected: true,
        })
    }

    pub(crate) fn take_child(&mut self) -> Option<Child> {
        self.child.take()
    }

    fn set_blinking_expected(&mut self, value: bool) {
        self.blinking_expected = value;
    }

    fn set_changing_expected(&mut self, value: bool) {
        self.changing_expected = value;
    }

    fn capture_full_screen(&self) -> anyhow::Result<()> {
        let screenshot = self.uitest_context.capture_full_screen()?;
        let path = self.test_case_dir.join(format!(
            "full_screen_{}",
            Utc::now().format("%d%m%Y%H%M%S.png")
        ));
        println!("saving full screen image to {path:?}");
        screenshot.save(path)?;
        Ok(())
    }

    fn capture_changed(&mut self, window: &Window) -> anyhow::Result<RgbaImage> {
        let mut started = Instant::now();
        let mut image = None;
        while started.elapsed() < MAX_CAPTURE_DURATION {
            let new_image = window.capture_image()?;
            if self.last_snapshots.get(&window.id()?) != Some(&new_image) {
                image = Some(new_image);
                break;
            }
            sleep(CAPTURE_INTERVAL);
        }
        if image.is_none() {
            self.capture_full_screen()?;
        }
        let mut image =
            image.context("expected a new snapshot, but no changes were detected in the window")?;
        started = Instant::now();
        while started.elapsed() < STATIONARY_INTERVAL {
            let new_image = window.capture_image()?;
            if new_image != image {
                image = new_image;
                started = Instant::now();
            }
            sleep(CAPTURE_INTERVAL);
        }
        self.last_snapshots.insert(window.id()?, image.clone());
        Ok(image)
    }

    fn capture_maybe_changed(&mut self, window: &Window) -> anyhow::Result<RgbaImage> {
        if self.changing_expected {
            self.capture_changed(window)
        } else {
            sleep(SIMPLE_CAPTURE_DELAY);
            let new_image = window.capture_image()?;
            self.last_snapshots.insert(window.id()?, new_image.clone());
            Ok(new_image)
        }
    }

    fn capture_blinking(&mut self, window: &Window, file_name: &str) -> anyhow::Result<RgbaImage> {
        let started = Instant::now();
        let mut images = Vec::new();
        while started.elapsed() < MAX_CAPTURE_DURATION || images.is_empty() {
            let new_image = self.capture_changed(window).with_context(|| {
                if images.is_empty() {
                    "failed to capture the first changed image while blinking was expected"
                } else {
                    "failed to capture the second changed image while blinking was expected"
                }
            })?;
            if !images.contains(&new_image) {
                images.push(new_image);
                if images.len() == 2 {
                    break;
                }
            }
            sleep(CAPTURE_INTERVAL);
        }
        images.sort_unstable_by(|a, b| a.as_raw().cmp(b.as_raw()));
        if images.len() == 2 {
            let b = images.pop().unwrap();
            let mut a = images.pop().unwrap();
            if a.dimensions() != b.dimensions() {
                bail!("unexpected screenshot size change");
            }
            let height_stride = a.sample_layout().height_stride;
            for y in (0..a.height() as usize).step_by(2) {
                (*a)[height_stride * y..height_stride * (y + 1)]
                    .copy_from_slice(&(*b)[height_stride * y..height_stride * (y + 1)]);
            }
            Ok(a)
        } else {
            record_fail(
                &mut self.fails,
                format!("expected blinking at {:?}", file_name),
            );
            assert_eq!(images.len(), 1);
            Ok(images.pop().unwrap())
        }
    }

    pub(crate) fn snapshot(&mut self, window: &Window, text: impl Display) -> anyhow::Result<()> {
        if !self.test_case_dir.try_exists()? {
            create_dir_all(&self.test_case_dir)?;
        }
        self.last_snapshot_index += 1;
        let index = self.last_snapshot_index;
        let confirmed_snapshot_name = format!("{:02} - {}.png", index, text);
        let unconfirmed_snapshot_name = format!("{:02} - {}.new.png", index, text);

        let new_image = if self.blinking_expected {
            self.capture_blinking(window, &unconfirmed_snapshot_name)
                .with_context(|| {
                    format!("failed to capture snapshot {confirmed_snapshot_name:?}")
                })?
        } else {
            self.capture_maybe_changed(window).with_context(|| {
                format!("failed to capture snapshot {confirmed_snapshot_name:?}")
            })?
        };
        let text = text.to_string();
        if !text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_')
        {
            bail!("disallowed char in snapshot text: {:?}", text);
        }

        let files = self.unverified_files.remove(&index).unwrap_or_default();
        if let Some(unconfirmed) = &files.unconfirmed {
            fs_err::remove_file(self.test_case_dir.join(&unconfirmed.full_name))?;
        }
        if let Some(confirmed) = &files.confirmed {
            let confirmed_image = load_image(&self.test_case_dir.join(&confirmed.full_name))?;
            if !snapshot_matches(&confirmed_image, &new_image) {
                let new_path = self.test_case_dir.join(&unconfirmed_snapshot_name);
                new_image
                    .save(&new_path)
                    .with_context(|| format!("failed to save image {:?}", &new_path))?;
                record_fail(
                    &mut self.fails,
                    format!("snapshot mismatch at {:?}", unconfirmed_snapshot_name),
                );
            } else if confirmed.full_name != confirmed_snapshot_name {
                fs_err::rename(
                    self.test_case_dir.join(&confirmed.full_name),
                    self.test_case_dir.join(&confirmed_snapshot_name),
                )?;
            }
        } else {
            let new_path = self.test_case_dir.join(&unconfirmed_snapshot_name);
            new_image
                .save(&new_path)
                .with_context(|| format!("failed to save image {:?}", &new_path))?;
            let fail = format!("new snapshot at {:?}", unconfirmed_snapshot_name);
            record_fail(&mut self.fails, fail);
        }
        Ok(())
    }

    fn finish(&mut self) -> Vec<String> {
        let extra_snapshots = self
            .unverified_files
            .values()
            .flat_map(|files| {
                files
                    .confirmed
                    .as_ref()
                    .into_iter()
                    .chain(&files.unconfirmed)
            })
            .map(|file| format!("{:?}", &file.full_name))
            .join(", ");
        if !extra_snapshots.is_empty() {
            record_fail(
                &mut self.fails,
                format!("extraneous snapshot files found: {}", extra_snapshots),
            );
        }
        mem::take(&mut self.fails)
    }

    fn wait_for_windows_by_pid_inner(
        &self,
        num_windows: usize,
    ) -> anyhow::Result<Vec<uitest::Window>> {
        let pid = self.pid.context("app has not been run yet")?;
        let started = Instant::now();
        let mut windows = Vec::new();
        while started.elapsed() < WAIT_FOR_WINDOWS_DURATION {
            windows = self.uitest_context.windows_by_pid(pid)?;
            if windows.len() == num_windows {
                return Ok(windows);
            }
            sleep(WAIT_FOR_WINDOWS_INTERVAL);
        }
        if windows.is_empty() {
            bail!(
                "couldn't find a window with pid={} after {:?}",
                pid,
                WAIT_FOR_WINDOWS_DURATION
            );
        } else if windows.len() > num_windows {
            bail!(
                "expected to find {} windows with pid={}, but found {} windows",
                num_windows,
                pid,
                windows.len(),
            );
        } else {
            bail!(
                "expected to find {} windows with pid={}, but found only {} windows after {:?}",
                num_windows,
                pid,
                windows.len(),
                WAIT_FOR_WINDOWS_DURATION
            );
        }
    }

    pub fn wait_for_windows_by_pid(
        &self,
        num_windows: usize,
    ) -> anyhow::Result<Vec<uitest::Window>> {
        let r = self.wait_for_windows_by_pid_inner(num_windows);
        if r.is_err() {
            self.capture_full_screen()?;
        }
        r
    }

    pub fn wait_for_window_by_pid(&self) -> anyhow::Result<uitest::Window> {
        let mut windows = self.wait_for_windows_by_pid(1)?;
        Ok(windows.remove(0))
    }

    pub fn test_subject_pid(&self) -> anyhow::Result<u32> {
        self.pid.context("app has not been started yet")
    }
}

fn record_fail(fails: &mut Vec<String>, fail: impl Display) {
    let fail = fail.to_string();
    println!("{fail}");
    fails.push(fail);
}

fn load_image(path: &Path) -> anyhow::Result<RgbaImage> {
    let reader =
        ImageReader::open(path).with_context(|| format!("failed to open image {:?}", path))?;
    let image = reader
        .decode()
        .with_context(|| format!("failed to decode image {:?}", path))?;
    Ok(image.into_rgba8())
}

enum ContextInner {
    Check(CheckContext),
    Run(Option<AppBuilder>),
}

#[derive(Clone)]
pub struct Context(Arc<Mutex<ContextInner>>);

impl Context {
    pub(crate) fn new_check(
        uitest_context: uitest::Context,
        test_name: String,
        test_case_dir: PathBuf,
        exe_path: OsString,
    ) -> anyhow::Result<Self> {
        Ok(Context(Arc::new(Mutex::new(ContextInner::Check(
            CheckContext::new(uitest_context, test_name, test_case_dir, exe_path)?,
        )))))
    }

    pub(crate) fn new_run(app: AppBuilder) -> Self {
        Context(Arc::new(Mutex::new(ContextInner::Run(Some(app)))))
    }

    pub(crate) fn check<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut CheckContext) -> R,
    {
        match &mut *self.0.lock().unwrap() {
            ContextInner::Check(ctx) => f(ctx),
            ContextInner::Run(_) => panic!("called a check function in a run context"),
        }
    }

    /// Runs a test subject program in a new process.
    ///
    /// The `init` function will be called to initialize the app.
    ///
    /// This function can only be called once per test.
    ///
    /// The PID of the spawned process will be available via [Context::test_subject_pid].
    pub fn run(
        &self,
        init: impl FnOnce(&mut RootWidget) -> anyhow::Result<()> + 'static,
    ) -> anyhow::Result<()> {
        match &mut *self.0.lock().unwrap() {
            ContextInner::Check(ctx) => {
                if ctx.pid.is_some() {
                    bail!("cannot run multiple test subjects in one test");
                }
                let child = Command::new(&ctx.exe_path)
                    .args(["run", &ctx.test_name])
                    .spawn()?;
                ctx.pid = Some(child.id());
                ctx.child = Some(child);
                Ok(())
            }
            ContextInner::Run(app) => {
                let app = app.take().context("cannot run multiple apps in one test")?;
                app.run(init)?;
                process::exit(0);
            }
        }
    }

    /// Set blinking detection behavior for subsequent snapshot captures.
    ///
    /// If `value` is `true`, snapshot capture functions will try to fetch
    /// two distinct snapshots of the window. If successful, these two snapshots
    /// will be combined to product a final snapshot. Use this setting to capture
    /// snapshots with a blinking text cursor.
    ///
    /// If `value` is `false`, only a single snapshot will be captured.
    ///
    /// The default is `false`.
    pub fn set_blinking_expected(&self, value: bool) {
        self.check(|c| c.set_blinking_expected(value));
    }

    /// Set change detection behavior for subsequent snapshot captures.
    ///
    ///
    /// If `value` is `true`, snapshot capture functions take the snapshots
    /// of the window repeatedly until a change has been detected.
    /// This is recommended in most cases because it makes tests more robust
    /// in virtual and CI environments.
    ///
    /// If `value` is `false`, a single snapshot will be captured after a fixed delay.
    ///
    /// The default is `true`.
    pub fn set_changing_expected(&self, value: bool) {
        self.check(|c| c.set_changing_expected(value));
    }

    pub(crate) fn finish(&self) -> Vec<String> {
        self.check(|c| c.finish())
    }

    pub fn wait_for_windows_by_pid(&self, num_windows: usize) -> anyhow::Result<Vec<Window>> {
        let windows = self.check(|c| c.wait_for_windows_by_pid(num_windows))?;
        Ok(windows
            .into_iter()
            .map(|inner| Window::new(inner, self.clone()))
            .collect())
    }

    pub fn wait_for_window_by_pid(&self) -> anyhow::Result<Window> {
        let inner = self.check(|c| c.wait_for_window_by_pid())?;
        Ok(Window::new(inner, self.clone()))
    }

    /// Returns process ID of the process launched by [Context::run].
    ///
    /// Returns an error if [Context::run] hasn't been called yet.
    pub fn test_subject_pid(&self) -> anyhow::Result<u32> {
        self.check(|c| c.test_subject_pid())
    }

    // TODO: remove?
    pub fn ui_context(&self) -> uitest::Context {
        self.check(|c| c.uitest_context.clone())
    }
}

fn snapshot_matches(a: &RgbaImage, b: &RgbaImage) -> bool {
    if a.dimensions() != b.dimensions() {
        return false;
    }

    a.pixels()
        .zip(b.pixels())
        .all(|(a, b)| a == b || a == &IGNORED_PIXEL || b == &IGNORED_PIXEL)
}
