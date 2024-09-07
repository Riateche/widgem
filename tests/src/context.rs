use std::{
    collections::BTreeMap,
    fmt::Display,
    path::{Path, PathBuf},
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{bail, Context as _};
use fs_err::read_dir;
use image::RgbaImage;
use itertools::Itertools;
use uitest::{Connection, Window};

#[derive(Debug, Default)]
struct SingleSnapshotFiles {
    confirmed: Option<String>,
    unconfirmed: Option<String>,
}

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
    ) -> anyhow::Result<Context> {
        let mut unverified_files = BTreeMap::<u32, SingleSnapshotFiles>::new();
        for entry in read_dir(&test_case_dir)? {
            let entry = entry?;
            let name = entry
                .file_name()
                .to_str()
                .with_context(|| {
                    format!("non-unicode file name in test case dir: {:?}", entry.path())
                })?
                .to_string();
            if !name.ends_with(".png") {
                continue;
            }
            let mut iter = name.splitn(2, " - ");
            let first = iter.next().expect("never fails");
            iter.next()
                .with_context(|| format!("invalid snapshot name: {:?}", entry.path()))?;
            let step: u32 = first
                .parse()
                .with_context(|| format!("invalid snapshot name: {:?}", entry.path()))?;
            let files = unverified_files.entry(step).or_default();
            if name.ends_with(".new.png") {
                if let Some(unconfirmed) = &files.unconfirmed {
                    bail!(
                        "duplicate unconfirmed files: {:?}, {:?}",
                        test_case_dir.join(unconfirmed),
                        entry.path()
                    );
                }
                files.unconfirmed = Some(name);
            } else {
                if let Some(confirmed) = &files.confirmed {
                    bail!(
                        "duplicate confirmed files: {:?}, {:?}",
                        test_case_dir.join(confirmed),
                        entry.path()
                    );
                }
                files.confirmed = Some(name);
            }
        }
        Ok(Self {
            connection,
            test_case_dir,
            pid,
            last_snapshot_index: 0,
            snapshot_mode,
            unverified_files,
            fails: Vec::new(),
            blinking_expected: false,
        })
    }

    pub fn snapshot(&mut self, window: &Window, text: impl Display) -> anyhow::Result<()> {
        let new_image = if self.blinking_expected {
            capture_blinking(window)?
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

        self.last_snapshot_index += 1;
        let index = self.last_snapshot_index;
        let confirmed_snapshot_name = format!("{} - {}.png", index, text);
        let unconfirmed_snapshot_name = format!("{} - {}.new.png", index, text);

        let files = self.unverified_files.remove(&index).unwrap_or_default();
        if let Some(unconfirmed) = &files.unconfirmed {
            fs_err::remove_file(self.test_case_dir.join(unconfirmed))?;
            if self.snapshot_mode == SnapshotMode::Check {
                record_fail(
                    &mut self.fails,
                    format!(
                        "unexpected unconfirmed snapshot: {:?}",
                        self.test_case_dir.join(unconfirmed),
                    ),
                );
            }
        }
        if let Some(confirmed) = &files.confirmed {
            let confirmed_image = load_image(&self.test_case_dir.join(confirmed))?;
            if confirmed != &confirmed_snapshot_name {
                fs_err::rename(
                    self.test_case_dir.join(confirmed),
                    self.test_case_dir.join(&confirmed_snapshot_name),
                )?;
                if self.snapshot_mode == SnapshotMode::Check {
                    record_fail(
                        &mut self.fails,
                        format!(
                            "confirmed snapshot name mismatch: expected {:?}, got {:?}",
                            self.test_case_dir.join(confirmed_snapshot_name),
                            self.test_case_dir.join(confirmed),
                        ),
                    );
                }
            }
            if confirmed_image != new_image {
                let new_path = self.test_case_dir.join(&unconfirmed_snapshot_name);
                new_image
                    .save(&new_path)
                    .with_context(|| format!("failed to save image {:?}", &new_path))?;
                record_fail(
                    &mut self.fails,
                    format!("snapshot mismatch at {:?}", new_path),
                );
            }
        } else {
            let new_path = self.test_case_dir.join(&unconfirmed_snapshot_name);
            new_image
                .save(&new_path)
                .with_context(|| format!("failed to save image {:?}", &new_path))?;
            let fail = match self.snapshot_mode {
                SnapshotMode::Update => format!("new snapshot at {:?}", new_path),
                SnapshotMode::Check => format!(
                    "missing snapshot at {:?}",
                    self.test_case_dir.join(confirmed_snapshot_name)
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
            .map(|name| format!("{:?}", self.test_case_dir.join(name)))
            .join(", ");
        if !extra_snapshots.is_empty() {
            record_fail(
                &mut self.fails,
                format!("extraneous snapshot files found: {}", extra_snapshots),
            );
        }
        self.fails
    }
}

fn record_fail(fails: &mut Vec<String>, fail: impl Display) {
    let fail = fail.to_string();
    println!("{fail}");
    fails.push(fail);
}

fn load_image(path: &Path) -> anyhow::Result<RgbaImage> {
    let reader = image::io::Reader::open(path)
        .with_context(|| format!("failed to open image {:?}", path))?;
    let image = reader
        .decode()
        .with_context(|| format!("failed to decode image {:?}", path))?;
    Ok(image.into_rgba8())
}

fn capture_blinking(window: &Window) -> anyhow::Result<RgbaImage> {
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
        assert_eq!(images.len(), 1);
        Ok(images.pop().unwrap())
    }
}
