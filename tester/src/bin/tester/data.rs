use std::{
    cmp::max,
    collections::BTreeMap,
    ops::Bound,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{ensure, Context as _};
use tiny_skia::PremultipliedColorU8;
use uitest::IGNORED_PIXEL;
use widgem::Pixmap;
use widgem_tester::{discover_snapshots, test_snapshots_dir, QueryAllResponse, SingleSnapshotFile};

#[derive(Debug)]
pub struct Position {
    pub test: String,
    pub snapshot: Option<u32>,
}

pub struct FileInfo {
    info: SingleSnapshotFile,
    pixmap: Option<Pixmap>,
}

pub struct Snapshot {
    confirmed: Option<FileInfo>,
    unconfirmed: Option<FileInfo>,
    diff_with_confirmed: Option<Pixmap>,
    diff_with_previous_confirmed: Option<Pixmap>,
}

impl Snapshot {
    fn confirmed_pixmap(&mut self) -> anyhow::Result<Option<Pixmap>> {
        let Some(confirmed) = &mut self.confirmed else {
            return Ok(None);
        };
        if let Some(pixmap) = &confirmed.pixmap {
            return Ok(Some(pixmap.clone()));
        }
        let pixmap = Pixmap::load_png(&confirmed.info.path)?;
        confirmed.pixmap = Some(pixmap.clone());
        Ok(Some(pixmap))
    }

    fn unconfirmed_pixmap(&mut self) -> anyhow::Result<Option<Pixmap>> {
        let Some(unconfirmed) = &mut self.unconfirmed else {
            return Ok(None);
        };
        if let Some(pixmap) = &unconfirmed.pixmap {
            return Ok(Some(pixmap.clone()));
        }
        let pixmap = Pixmap::load_png(&unconfirmed.info.path)?;
        unconfirmed.pixmap = Some(pixmap.clone());
        Ok(Some(pixmap))
    }

    fn approve(&mut self) -> anyhow::Result<()> {
        let unconfirmed = self.unconfirmed.as_ref().context("no unconfirmed file")?;

        if let Some(confirmed) = self.confirmed.take() {
            fs_err::remove_file(&confirmed.info.path)?;
        }
        let unsuffixed = unconfirmed
            .info
            .full_name
            .strip_suffix(".new.png")
            .context("invalid unconfirmed file name")?;
        let confirmed_name = format!("{unsuffixed}.png");
        let test_case_dir = unconfirmed.info.path.parent().context("no parent path")?;
        let confirmed_path = test_case_dir.join(&confirmed_name);
        fs_err::rename(&unconfirmed.info.path, &confirmed_path)?;
        self.confirmed = Some(FileInfo {
            info: SingleSnapshotFile {
                modified: fs_err::metadata(&confirmed_path)?.modified()?,
                path: confirmed_path,
                description: unconfirmed.info.description.clone(),
                full_name: confirmed_name,
            },
            pixmap: unconfirmed.pixmap.clone(),
        });
        self.unconfirmed = None;
        Ok(())
    }
}

pub struct Test {
    name: String,
    index: usize,
    snapshots: BTreeMap<u32, Snapshot>,
}

pub struct Config {
    pub tests_dir: PathBuf,
    pub run_script: Option<PathBuf>,
}

pub struct Tests {
    config: Config,
    snapshots_dir: PathBuf,
    test_names: Vec<String>,
    tests: BTreeMap<String, Test>,
}

impl Tests {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let mut tests = Tests {
            config,
            // Will be initialized immediately in refresh()
            snapshots_dir: "".into(),
            test_names: Vec::new(),
            tests: BTreeMap::new(),
        };
        tests.refresh()?;
        Ok(tests)
    }

    pub fn refresh(&mut self) -> anyhow::Result<()> {
        let data = query_data(&self.config.tests_dir)?;
        self.snapshots_dir = data.snapshots_dir;
        // Delete old tests that don't exist anymore.
        self.tests.retain(|name, _| data.test_cases.contains(name));
        for (index, test_case) in data.test_cases.iter().enumerate() {
            let test = self.tests.entry(test_case.clone()).or_insert_with(|| Test {
                name: test_case.clone(),
                index,
                snapshots: BTreeMap::new(),
            });
            test.index = index;
            let new_snapshots =
                discover_snapshots(&test_snapshots_dir(&self.snapshots_dir, test_case))?;
            // Delete old snapshots that don't exist anymore.
            test.snapshots
                .retain(|number, _| new_snapshots.contains_key(number));
            for old_snapshot in test.snapshots.values_mut() {
                old_snapshot.diff_with_previous_confirmed = None;
            }
            for (number, new_snapshot) in new_snapshots {
                let snapshot = test.snapshots.entry(number).or_insert_with(|| Snapshot {
                    confirmed: None,
                    unconfirmed: None,
                    diff_with_confirmed: None,
                    diff_with_previous_confirmed: None,
                });
                if !is_same_file_info(&snapshot.unconfirmed, &new_snapshot.unconfirmed) {
                    snapshot.unconfirmed = new_snapshot
                        .unconfirmed
                        .map(|info| FileInfo { info, pixmap: None });
                    snapshot.diff_with_confirmed = None;
                    snapshot.diff_with_previous_confirmed = None;
                }
                if !is_same_file_info(&snapshot.confirmed, &new_snapshot.confirmed) {
                    snapshot.confirmed = new_snapshot
                        .confirmed
                        .map(|info| FileInfo { info, pixmap: None });
                    snapshot.diff_with_confirmed = None;
                }
            }
        }

        self.test_names = data.test_cases;
        Ok(())
    }

    pub fn has_unconfirmed_snapshots(&self) -> bool {
        self.tests.values().any(|test| {
            test.snapshots
                .values()
                .any(|snapshot| snapshot.unconfirmed.is_some())
        })
    }

    pub fn unconfirmed_snapshot_count(&self) -> usize {
        self.tests
            .values()
            .flat_map(|test| test.snapshots.values())
            .filter(|snapshot| snapshot.unconfirmed.is_some())
            .count()
    }

    pub fn num_tests(&self) -> usize {
        self.tests.len()
    }

    pub fn num_snapshots(&self, test: &str) -> usize {
        self.tests.get(test).map_or(0, |test| test.snapshots.len())
    }

    pub fn next_unconfirmed_pos(&self, from: Option<&Position>) -> Option<Position> {
        if let Some(from) = from {
            if let Some(test) = self.tests.get(&from.test) {
                let from_snapshot = if let Some(from) = from.snapshot {
                    Bound::Excluded(from)
                } else {
                    Bound::Unbounded
                };
                for (number, snapshot) in test.snapshots.range((from_snapshot, Bound::Unbounded)) {
                    if snapshot.unconfirmed.is_some() {
                        return Some(Position {
                            test: from.test.clone(),
                            snapshot: Some(*number),
                        });
                    }
                }
            }
        }

        let from_test = if let Some(from) = from {
            Bound::Excluded(&from.test)
        } else {
            Bound::Unbounded
        };
        for (test_name, test) in self.tests.range::<String, _>((from_test, Bound::Unbounded)) {
            for (number, snapshot) in &test.snapshots {
                if snapshot.unconfirmed.is_some() {
                    return Some(Position {
                        test: test_name.clone(),
                        snapshot: Some(*number),
                    });
                }
            }
        }
        None
    }

    pub fn next_test(&self, from: Option<&Position>) -> Option<Position> {
        let from_test = if let Some(from) = from {
            Bound::Excluded(&from.test)
        } else {
            Bound::Unbounded
        };
        let (test_name, test) = self
            .tests
            .range::<String, _>((from_test, Bound::Unbounded))
            .next()?;
        Some(Position {
            test: test_name.clone(),
            snapshot: test.snapshots.keys().next().copied(),
        })
    }

    pub fn previous_test(&self, from: Option<&Position>) -> Option<Position> {
        let from_test = if let Some(from) = from {
            Bound::Excluded(&from.test)
        } else {
            Bound::Unbounded
        };
        let (test_name, test) = self
            .tests
            .range::<String, _>((Bound::Unbounded, from_test))
            .next_back()?;
        Some(Position {
            test: test_name.clone(),
            snapshot: test.snapshots.keys().next().copied(),
        })
    }

    pub fn next_snapshot(&self, from: &Position) -> Option<Position> {
        let test = self.tests.get(&from.test)?;
        let from_snapshot = if let Some(from) = from.snapshot {
            Bound::Excluded(from)
        } else {
            Bound::Unbounded
        };
        let (number, _) = test
            .snapshots
            .range((from_snapshot, Bound::Unbounded))
            .next()?;
        Some(Position {
            test: from.test.clone(),
            snapshot: Some(*number),
        })
    }

    pub fn previous_snapshot(&self, from: &Position) -> Option<Position> {
        let test = self.tests.get(&from.test)?;
        let from_snapshot = if let Some(from) = from.snapshot {
            Bound::Excluded(from)
        } else {
            Bound::Unbounded
        };
        let (number, _) = test
            .snapshots
            .range((Bound::Unbounded, from_snapshot))
            .next_back()?;
        Some(Position {
            test: from.test.clone(),
            snapshot: Some(*number),
        })
    }

    pub fn has_confirmed(&self, pos: &Position) -> bool {
        pos.snapshot
            .and_then(|number| {
                self.tests
                    .get(&pos.test)
                    .and_then(|test| test.snapshots.get(&number))
            })
            .and_then(|snapshot| snapshot.confirmed.as_ref())
            .is_some()
    }

    pub fn has_unconfirmed(&self, pos: &Position) -> bool {
        pos.snapshot
            .and_then(|number| {
                self.tests
                    .get(&pos.test)
                    .and_then(|test| test.snapshots.get(&number))
            })
            .and_then(|snapshot| snapshot.unconfirmed.as_ref())
            .is_some()
    }

    pub fn unconfirmed_description(&self, pos: &Position) -> Option<&str> {
        pos.snapshot
            .and_then(|number| {
                self.tests
                    .get(&pos.test)
                    .and_then(|test| test.snapshots.get(&number))
            })
            .and_then(|snapshot| snapshot.unconfirmed.as_ref())
            .map(|file| file.info.description.as_str())
    }

    pub fn confirmed_description(&self, pos: &Position) -> Option<&str> {
        pos.snapshot
            .and_then(|number| {
                self.tests
                    .get(&pos.test)
                    .and_then(|test| test.snapshots.get(&number))
            })
            .and_then(|snapshot| snapshot.confirmed.as_ref())
            .map(|file| file.info.description.as_str())
    }

    pub fn test_index(&self, test_name: &str) -> Option<usize> {
        self.tests.get(test_name).map(|test| test.index)
    }

    pub fn unconfirmed_pixmap(&mut self, pos: &Position) -> anyhow::Result<Option<Pixmap>> {
        let Some(test) = self.tests.get_mut(&pos.test) else {
            return Ok(None);
        };
        let Some(number) = pos.snapshot else {
            return Ok(None);
        };
        let Some(snapshot) = test.snapshots.get_mut(&number) else {
            return Ok(None);
        };
        snapshot.unconfirmed_pixmap()
    }

    pub fn confirmed_pixmap(&mut self, pos: &Position) -> anyhow::Result<Option<Pixmap>> {
        let Some(test) = self.tests.get_mut(&pos.test) else {
            return Ok(None);
        };
        let Some(number) = pos.snapshot else {
            return Ok(None);
        };
        let Some(snapshot) = test.snapshots.get_mut(&number) else {
            return Ok(None);
        };
        snapshot.confirmed_pixmap()
    }

    pub fn diff_with_confirmed(&mut self, pos: &Position) -> anyhow::Result<Option<Pixmap>> {
        let Some(test) = self.tests.get_mut(&pos.test) else {
            return Ok(None);
        };
        let Some(number) = pos.snapshot else {
            return Ok(None);
        };
        let Some(snapshot) = test.snapshots.get_mut(&number) else {
            return Ok(None);
        };
        if let Some(pixmap) = &snapshot.diff_with_confirmed {
            return Ok(Some(pixmap.clone()));
        }
        let Some(a) = snapshot.unconfirmed_pixmap()? else {
            return Ok(None);
        };
        let Some(b) = snapshot.confirmed_pixmap()? else {
            return Ok(None);
        };
        let pixmap = pixmap_diff(&a, &b);
        snapshot.diff_with_confirmed = Some(pixmap.clone());
        Ok(Some(pixmap))
    }

    pub fn diff_with_previous_confirmed(
        &mut self,
        pos: &Position,
    ) -> anyhow::Result<Option<Pixmap>> {
        let Some(prev_number) = self.previous_snapshot(pos).and_then(|pos| pos.snapshot) else {
            return Ok(None);
        };
        let Some(test) = self.tests.get_mut(&pos.test) else {
            return Ok(None);
        };
        let Some(number) = pos.snapshot else {
            return Ok(None);
        };
        let Some(snapshot) = test.snapshots.get_mut(&number) else {
            return Ok(None);
        };
        if let Some(pixmap) = &snapshot.diff_with_previous_confirmed {
            return Ok(Some(pixmap.clone()));
        }
        let Some(a) = snapshot.unconfirmed_pixmap()? else {
            return Ok(None);
        };
        let Some(prev_snapshot) = test.snapshots.get_mut(&prev_number) else {
            return Ok(None);
        };
        let Some(b) = prev_snapshot.confirmed_pixmap()? else {
            return Ok(None);
        };
        let pixmap = pixmap_diff(&a, &b);
        let Some(snapshot) = test.snapshots.get_mut(&number) else {
            return Ok(None);
        };
        snapshot.diff_with_previous_confirmed = Some(pixmap.clone());
        Ok(Some(pixmap))
    }

    pub fn approve(&mut self, pos: &Position) -> anyhow::Result<()> {
        let test = self.tests.get_mut(&pos.test).context("no such test")?;
        let number = pos.snapshot.context("no current snapshot")?;
        let snapshot = test
            .snapshots
            .get_mut(&number)
            .context("no such snapshot")?;
        snapshot.approve()?;
        Ok(())
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn closest_valid_pos(&self, from: &Position) -> Option<Position> {
        let test = self
            .tests
            .get(&from.test)
            .or_else(|| {
                self.tests
                    .range::<String, _>((Bound::Excluded(&from.test), Bound::Unbounded))
                    .next()
                    .map(|(_k, v)| v)
            })
            .or_else(|| {
                self.tests
                    .range::<String, _>((Bound::Unbounded, Bound::Excluded(&from.test)))
                    .next_back()
                    .map(|(_k, v)| v)
            })
            .or_else(|| self.tests.iter().next().map(|(_k, v)| v))?;

        let snapshot = if let Some(from) = from.snapshot {
            Some(from)
                .filter(|_| test.snapshots.contains_key(&from))
                .or_else(|| {
                    test.snapshots
                        .range((Bound::Excluded(&from), Bound::Unbounded))
                        .next()
                        .map(|(k, _v)| *k)
                })
                .or_else(|| {
                    test.snapshots
                        .range((Bound::Unbounded, Bound::Excluded(&from)))
                        .next_back()
                        .map(|(k, _v)| *k)
                })
                .or_else(|| test.snapshots.iter().next().map(|(k, _v)| *k))
        } else {
            test.snapshots.iter().next().map(|(k, _v)| *k)
        };
        Some(Position {
            test: test.name.clone(),
            snapshot,
        })
    }
}

fn is_same_file_info(a: &Option<FileInfo>, b: &Option<SingleSnapshotFile>) -> bool {
    match (a, b) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some(_), None) => false,
        (Some(a), Some(b)) => &a.info == b,
    }
}

fn pixmap_diff(a: &Pixmap, b: &Pixmap) -> Pixmap {
    let a = a.as_tiny_skia_ref();
    let b = b.as_tiny_skia_ref();
    let mut out =
        tiny_skia::Pixmap::new(max(a.width(), b.width()), max(a.height(), b.height())).unwrap();
    let width = out.width();
    let ignored_pixel = PremultipliedColorU8::from_rgba(
        IGNORED_PIXEL[0],
        IGNORED_PIXEL[1],
        IGNORED_PIXEL[2],
        IGNORED_PIXEL[3],
    )
    .unwrap();
    for y in 0..out.height() {
        for x in 0..width {
            let pixel_a = if x < a.width() && y < a.height() {
                a.pixel(x, y)
            } else {
                None
            };
            let pixel_b = if x < b.width() && y < b.height() {
                b.pixel(x, y)
            } else {
                None
            };
            let pixel_out = match (pixel_a, pixel_b) {
                (None, None) => PremultipliedColorU8::from_rgba(0, 0, 0, 0).unwrap(),
                (None, Some(_)) => PremultipliedColorU8::from_rgba(0, 0, 255, 255).unwrap(),
                (Some(_), None) => PremultipliedColorU8::from_rgba(255, 0, 255, 255).unwrap(),
                (Some(pixel_a), Some(pixel_b)) => {
                    if pixel_a == pixel_b || pixel_b == ignored_pixel {
                        pixel_a
                    } else if pixel_a == ignored_pixel {
                        pixel_b
                    } else {
                        //     PremultipliedColorU8::from_rgba(
                        //         u8_diff(pixel_a.red(), pixel_b.red()),
                        //         u8_diff(pixel_a.green(), pixel_b.green()),
                        //         u8_diff(pixel_a.blue(), pixel_b.blue()),
                        //         255,
                        //     )
                        //     .unwrap()
                        PremultipliedColorU8::from_rgba(255, 0, 0, 255).unwrap()
                    }
                }
            };
            out.pixels_mut()[(y * width + x) as usize] = pixel_out;
        }
    }

    out.into()
}

pub fn query_data(path: &Path) -> anyhow::Result<QueryAllResponse> {
    let output = Command::new("cargo")
        .args(["run", "--", "query", "all"])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .current_dir(path)
        .output()?;
    ensure!(output.status.success(), "failed to run cargo");
    let data = serde_json::from_slice::<QueryAllResponse>(&output.stdout).with_context(|| {
        format!(
            "couldn't parse output: {:?}",
            String::from_utf8_lossy(&output.stdout)
        )
    })?;
    Ok(data)
}
