use {
    crate::{discover_snapshots, repo_dir, SingleSnapshotFiles},
    anyhow::{bail, Context as _},
    fs_err::create_dir,
    image::{ImageReader, RgbaImage},
    itertools::Itertools,
    std::{
        collections::BTreeMap,
        fmt::Display,
        path::{Path, PathBuf},
        thread::sleep,
        time::{Duration, Instant},
    },
    uitest::{Connection, Window},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnapshotMode {
    Update,
    Check,
}

pub struct Context<'a> {
    pub connection: &'a mut Connection,
    pub test_case_dir: PathBuf,
    pub last_snapshot_index: u32,
    pub snapshot_mode: SnapshotMode,
    pub pid: u32,
    unverified_files: BTreeMap<u32, SingleSnapshotFiles>,
    fails: Vec<String>,
    pub blinking_expected: bool,
}

impl<'a> Context<'a> {
    pub fn new(
        connection: &'a mut Connection,
        test_case_dir: PathBuf,
        snapshot_mode: SnapshotMode,
        pid: u32,
    ) -> anyhow::Result<Context<'a>> {
        Ok(Self {
            unverified_files: discover_snapshots(&test_case_dir)?,
            connection,
            test_case_dir,
            pid,
            last_snapshot_index: 0,
            snapshot_mode,
            fails: Vec::new(),
            blinking_expected: false,
        })
    }

    pub fn set_blinking_expected(&mut self, value: bool) {
        self.blinking_expected = value;
    }

    fn capture_blinking(
        &mut self,
        window: &mut Window,
        file_name: &str,
    ) -> anyhow::Result<RgbaImage> {
        const CAPTURE_INTERVAL: Duration = Duration::from_millis(100);
        const MAX_DURATION: Duration = Duration::from_secs(2);

        let started = Instant::now();
        let mut images = Vec::new();
        while started.elapsed() < MAX_DURATION || images.is_empty() {
            let new_image = window.capture_image()?;
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
                format!(
                    "expected blinking at {:?}",
                    self.test_case_dir
                        .join(file_name)
                        .strip_prefix(repo_dir())
                        .expect("failed to strip path prefix")
                ),
            );
            assert_eq!(images.len(), 1);
            Ok(images.pop().unwrap())
        }
    }

    pub fn snapshot(&mut self, window: &mut Window, text: impl Display) -> anyhow::Result<()> {
        if !self.test_case_dir.try_exists()? {
            create_dir(&self.test_case_dir)?;
        }
        sleep(Duration::from_millis(500));
        self.last_snapshot_index += 1;
        let index = self.last_snapshot_index;
        let confirmed_snapshot_name = format!("{:02} - {}.png", index, text);
        let unconfirmed_snapshot_name = format!("{:02} - {}.new.png", index, text);

        let new_image = if self.blinking_expected {
            self.capture_blinking(window, &unconfirmed_snapshot_name)?
        } else {
            window.capture_image()?
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
                        self.test_case_dir
                            .join(&unconfirmed.full_name)
                            .strip_prefix(repo_dir())
                            .expect("failed to strip path prefix"),
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
                    format!(
                        "snapshot mismatch at {:?}",
                        new_path
                            .strip_prefix(repo_dir())
                            .expect("failed to strip path prefix")
                    ),
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
                            self.test_case_dir
                                .join(confirmed_snapshot_name)
                                .strip_prefix(repo_dir())
                                .expect("failed to strip path prefix"),
                            self.test_case_dir
                                .join(&confirmed.full_name)
                                .strip_prefix(repo_dir())
                                .expect("failed to strip path prefix"),
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
                SnapshotMode::Update => format!(
                    "new snapshot at {:?}",
                    new_path
                        .strip_prefix(repo_dir())
                        .expect("failed to strip path prefix")
                ),
                SnapshotMode::Check => format!(
                    "missing snapshot at {:?}",
                    self.test_case_dir
                        .join(confirmed_snapshot_name)
                        .strip_prefix(repo_dir())
                        .expect("failed to strip path prefix")
                ),
            };
            record_fail(&mut self.fails, fail);
        }
        Ok(())
    }

    pub fn finish(mut self) -> Vec<String> {
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
            .map(|file| {
                format!(
                    "{:?}",
                    self.test_case_dir
                        .join(&file.full_name)
                        .strip_prefix(repo_dir())
                        .expect("failed to strip path prefix")
                )
            })
            .join(", ");
        if !extra_snapshots.is_empty() {
            record_fail(
                &mut self.fails,
                format!("extraneous snapshot files found: {}", extra_snapshots),
            );
        }
        self.fails
    }

    pub fn wait_for_windows_by_pid(&self) -> anyhow::Result<Vec<Window>> {
        self.connection.wait_for_windows_by_pid(self.pid)
    }

    pub fn wait_for_window_by_pid(&self) -> anyhow::Result<Window> {
        let mut windows = self.connection.wait_for_windows_by_pid(self.pid)?;
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
