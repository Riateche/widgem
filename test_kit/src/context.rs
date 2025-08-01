use {
    crate::{discover_snapshots, SingleSnapshotFiles},
    anyhow::{bail, Context as _},
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
        thread::sleep,
        time::{Duration, Instant},
    },
    uitest::{Connection, Window},
    widgem::{widgets::RootWidget, App},
};

const CAPTURE_INTERVAL: Duration = Duration::from_millis(30);
const MAX_DURATION: Duration = Duration::from_secs(2);
const STATIONARY_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnapshotMode {
    Update,
    Check,
}

pub struct CheckContext {
    pub connection: Connection,
    pub test_name: String,
    pub test_case_dir: PathBuf,
    pub last_snapshot_index: u32,
    pub snapshot_mode: SnapshotMode,
    pub exe_path: OsString,
    pub pid: Option<u32>,
    pub child: Option<Child>,
    unverified_files: BTreeMap<u32, SingleSnapshotFiles>,
    fails: Vec<String>,
    pub blinking_expected: bool,
    pub changing_expected: bool,
    pub last_snapshots: BTreeMap<u32, RgbaImage>,
}

impl CheckContext {
    pub fn new(
        connection: Connection,
        test_name: String,
        test_case_dir: PathBuf,
        snapshot_mode: SnapshotMode,
        exe_path: OsString,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            unverified_files: discover_snapshots(&test_case_dir)?,
            connection,
            test_name,
            test_case_dir,
            exe_path,
            pid: None,
            child: None,
            last_snapshot_index: 0,
            last_snapshots: BTreeMap::new(),
            snapshot_mode,
            fails: Vec::new(),
            blinking_expected: false,
            changing_expected: true,
        })
    }

    pub fn set_blinking_expected(&mut self, value: bool) {
        self.blinking_expected = value;
    }

    pub fn set_changing_expected(&mut self, value: bool) {
        self.changing_expected = value;
    }

    fn capture_changed(&mut self, window: &mut Window) -> anyhow::Result<RgbaImage> {
        let mut started = Instant::now();
        let mut image = None;
        while started.elapsed() < MAX_DURATION {
            let new_image = window.capture_image()?;
            if self.last_snapshots.get(&window.pid()) != Some(&new_image) {
                image = Some(new_image);
                break;
            }
            sleep(CAPTURE_INTERVAL);
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
        self.last_snapshots.insert(window.pid(), image.clone());
        Ok(image)
    }

    fn capture_maybe_changed(&mut self, window: &mut Window) -> anyhow::Result<RgbaImage> {
        if self.changing_expected {
            self.capture_changed(window)
        } else {
            sleep(Duration::from_millis(500));
            let new_image = window.capture_image()?;
            self.last_snapshots.insert(window.pid(), new_image.clone());
            Ok(new_image)
        }
    }

    fn capture_blinking(
        &mut self,
        window: &mut Window,
        file_name: &str,
    ) -> anyhow::Result<RgbaImage> {
        let started = Instant::now();
        let mut images = Vec::new();
        while started.elapsed() < MAX_DURATION || images.is_empty() {
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

    pub fn snapshot(&mut self, window: &mut Window, text: impl Display) -> anyhow::Result<()> {
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
            if self.snapshot_mode == SnapshotMode::Check {
                record_fail(
                    &mut self.fails,
                    format!(
                        "unexpected unconfirmed snapshot: {:?}",
                        &unconfirmed.full_name,
                    ),
                );
            }
        }
        if let Some(confirmed) = &files.confirmed {
            let confirmed_image = load_image(&self.test_case_dir.join(&confirmed.full_name))?;
            if confirmed_image != new_image {
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
                if self.snapshot_mode == SnapshotMode::Check {
                    record_fail(
                        &mut self.fails,
                        format!(
                            "confirmed snapshot name mismatch: expected {:?}, got {:?}",
                            confirmed_snapshot_name, &confirmed.full_name,
                        ),
                    );
                }
            }
        } else {
            let new_path = self.test_case_dir.join(&unconfirmed_snapshot_name);
            new_image
                .save(&new_path)
                .with_context(|| format!("failed to save image {:?}", &new_path))?;
            let fail = match self.snapshot_mode {
                SnapshotMode::Update => format!("new snapshot at {:?}", unconfirmed_snapshot_name),
                SnapshotMode::Check => format!("missing snapshot at {:?}", confirmed_snapshot_name),
            };
            record_fail(&mut self.fails, fail);
        }
        Ok(())
    }

    pub fn finish(&mut self) -> Vec<String> {
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

    pub fn wait_for_windows_by_pid(&self) -> anyhow::Result<Vec<Window>> {
        let pid = self.pid.context("app has not been run yet")?;
        self.connection.wait_for_windows_by_pid(pid)
    }

    pub fn wait_for_window_by_pid(&self) -> anyhow::Result<Window> {
        let pid = self.pid.context("app has not been run yet")?;
        let mut windows = self.connection.wait_for_windows_by_pid(pid)?;
        if windows.len() != 1 {
            bail!("expected 1 window, got {}", windows.len());
        }
        Ok(windows.remove(0))
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

pub enum Context {
    Check(Box<CheckContext>),
    Run(Option<App>),
}

impl Context {
    pub(crate) fn as_check(&mut self) -> &mut CheckContext {
        match self {
            Context::Check(ctx) => ctx,
            Context::Run(_) => panic!("called a check function in a run context"),
        }
    }

    pub fn run(
        &mut self,
        init: impl FnOnce(&mut RootWidget) -> anyhow::Result<()> + 'static,
    ) -> anyhow::Result<()> {
        match self {
            Context::Check(ctx) => {
                let child = Command::new(&ctx.exe_path)
                    .args(["run", &ctx.test_name])
                    .spawn()?;
                ctx.pid = Some(child.id());
                ctx.child = Some(child);
                Ok(())
            }
            Context::Run(app) => {
                let app = app.take().context("cannot run multiple apps in one test")?;
                app.run(init)?;
                process::exit(0);
            }
        }
    }

    pub fn set_blinking_expected(&mut self, value: bool) {
        self.as_check().set_blinking_expected(value);
    }

    pub fn set_changing_expected(&mut self, value: bool) {
        self.as_check().set_changing_expected(value);
    }

    pub fn snapshot(&mut self, window: &mut Window, text: impl Display) -> anyhow::Result<()> {
        self.as_check().snapshot(window, text)
    }

    pub fn finish(&mut self) -> Vec<String> {
        self.as_check().finish()
    }

    pub fn wait_for_windows_by_pid(&mut self) -> anyhow::Result<Vec<Window>> {
        self.as_check().wait_for_windows_by_pid()
    }

    pub fn wait_for_window_by_pid(&mut self) -> anyhow::Result<Window> {
        self.as_check().wait_for_window_by_pid()
    }

    pub fn connection(&mut self) -> &mut Connection {
        &mut self.as_check().connection
    }
}
