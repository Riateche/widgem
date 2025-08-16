use {
    anyhow::{ensure, Context},
    std::{
        cmp::max,
        collections::BTreeMap,
        path::{Path, PathBuf},
        process::{self, Command, Stdio},
    },
    strum::EnumIter,
    tiny_skia::{Pixmap, PremultipliedColorU8},
    tracing::{info, warn},
    widgem_tester::{
        discover_snapshots, test_snapshots_dir, QueryAllResponse, SingleSnapshotFile,
        SingleSnapshotFiles,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum Mode {
    New,
    Confirmed,
    DiffWithConfirmed,
    DiffWithPreviousConfirmed,
}

pub struct TesterLogic {
    tests_dir: PathBuf,
    run_script: Option<PathBuf>,
    snapshots_dir: PathBuf,
    mode: Mode,
    test_cases: Vec<String>,
    current_test_case_index: Option<usize>,
    all_snapshots: Vec<BTreeMap<u32, SingleSnapshotFiles>>,
    current_snapshot_index: Option<u32>,
}

impl TesterLogic {
    pub fn new(tests_dir: PathBuf, run_script: Option<PathBuf>) -> anyhow::Result<Self> {
        let data = query_data(&tests_dir)?;
        let mut all_snapshots = Vec::new();
        for test_case in &data.test_cases {
            all_snapshots.push(
                discover_snapshots(&test_snapshots_dir(&data.snapshots_dir, test_case))
                    .unwrap_or_else(|err| {
                        // TODO: ui message
                        warn!("failed to fetch snapshots: {:?}", err);
                        Default::default()
                    }),
            );
        }
        let mut this = Self {
            tests_dir,
            run_script,
            snapshots_dir: data.snapshots_dir,
            mode: Mode::New,
            test_cases: data.test_cases,
            current_test_case_index: None,
            all_snapshots,
            current_snapshot_index: None,
        };
        this.go_to_next_test_case();
        Ok(this)
    }

    pub fn test_cases(&self) -> &[String] {
        &self.test_cases
    }

    #[allow(clippy::collapsible_if)]
    pub fn go_to_next_unconfirmed_file(&mut self) -> bool {
        loop {
            if !self.go_to_next_snapshot() {
                if !self.go_to_next_test_case() {
                    return false;
                }
            }
            if self
                .current_snapshot()
                .is_some_and(|f| f.unconfirmed.is_some())
            {
                return true;
            }
        }
    }

    pub fn go_to_next_test_case(&mut self) -> bool {
        let index = self.current_test_case_index.map_or(0, |i| i + 1);
        self.go_to_test_case(index)
    }

    pub fn go_to_previous_test_case(&mut self) -> bool {
        if self.current_test_case_index == Some(0) {
            return false;
        }
        let index = self.current_test_case_index.map_or(0, |i| i - 1);
        self.go_to_test_case(index)
    }

    pub fn go_to_test_case(&mut self, index: usize) -> bool {
        self.current_snapshot_index = None;
        if index >= self.test_cases.len() {
            return false;
        }
        self.current_test_case_index = Some(index);
        self.go_to_next_snapshot();
        true
    }

    pub fn go_to_previous_snapshot(&mut self) -> bool {
        if self.current_snapshot_index == Some(1) {
            return false;
        }
        let index = self.current_snapshot_index.map_or(0, |i| i - 1);
        self.go_to_snapshot(index)
    }

    pub fn go_to_next_snapshot(&mut self) -> bool {
        let index = self.current_snapshot_index.map_or(1, |i| i + 1);
        self.go_to_snapshot(index)
    }

    pub fn go_to_snapshot(&mut self, index: u32) -> bool {
        let Some(snapshots) = self
            .current_test_case_index
            .and_then(|index| self.all_snapshots.get(index))
        else {
            warn!(
                "invalid current_test_case_index {:?}",
                self.current_test_case_index
            );
            return false;
        };
        let Some(files) = snapshots.get(&index) else {
            return false;
        };
        self.current_snapshot_index = Some(index);
        if !self.is_mode_allowed(self.mode) {
            self.mode = if files.unconfirmed.is_some() {
                Mode::New
            } else {
                Mode::Confirmed
            };
        }
        true
    }

    pub fn current_test_case_name(&self) -> Option<&str> {
        Some(self.test_cases.get(self.current_test_case_index?)?.as_str())
    }

    fn previous_snapshot(&self) -> Option<&SingleSnapshotFiles> {
        let index = self.current_snapshot_index?.checked_sub(1)?;
        self.all_snapshots
            .get(self.current_test_case_index?)?
            .get(&index)
    }

    pub fn current_snapshot(&self) -> Option<&SingleSnapshotFiles> {
        self.all_snapshots
            .get(self.current_test_case_index?)?
            .get(&self.current_snapshot_index?)
    }

    fn current_snapshot_mut(&mut self) -> anyhow::Result<&mut SingleSnapshotFiles> {
        self.all_snapshots
            .get_mut(
                self.current_test_case_index
                    .context("no current_test_case_index")?,
            )
            .context("invalid current_test_case_index")?
            .get_mut(&self.current_snapshot_index.context("no current files")?)
            .context("current files not found")
    }

    fn load_new(&self) -> anyhow::Result<Option<Pixmap>> {
        let Some(test_case) = self.current_test_case_name() else {
            return Ok(None);
        };
        let Some(snapshot) = self.current_snapshot() else {
            return Ok(None);
        };
        let Some(unconfirmed) = snapshot.unconfirmed.as_ref() else {
            return Ok(None);
        };
        let path = test_snapshots_dir(&self.snapshots_dir, test_case).join(&unconfirmed.full_name);
        Ok(Some(Pixmap::load_png(path)?))
    }

    fn load_confirmed(&self) -> anyhow::Result<Option<Pixmap>> {
        let Some(test_case) = self.current_test_case_name() else {
            return Ok(None);
        };
        let Some(snapshot) = self.current_snapshot() else {
            return Ok(None);
        };
        let Some(confirmed) = snapshot.confirmed.as_ref() else {
            return Ok(None);
        };
        let path = test_snapshots_dir(&self.snapshots_dir, test_case).join(&confirmed.full_name);
        Ok(Some(Pixmap::load_png(path)?))
    }

    fn load_previous_confirmed(&self) -> anyhow::Result<Option<Pixmap>> {
        let Some(test_case) = self.current_test_case_name() else {
            return Ok(None);
        };
        let Some(snapshot) = self.previous_snapshot() else {
            return Ok(None);
        };
        let Some(confirmed) = snapshot.confirmed.as_ref() else {
            return Ok(None);
        };
        let path = test_snapshots_dir(&self.snapshots_dir, test_case).join(&confirmed.full_name);
        Ok(Some(Pixmap::load_png(path)?))
    }

    pub fn pixmap(&self) -> anyhow::Result<Option<Pixmap>> {
        match self.mode {
            Mode::New => self.load_new(),
            Mode::Confirmed => self.load_confirmed(),
            Mode::DiffWithConfirmed => {
                let Some(first) = self.load_new()? else {
                    return Ok(None);
                };
                let Some(second) = self.load_confirmed()? else {
                    return Ok(None);
                };
                Ok(Some(pixmap_diff(&first, &second)))
            }
            Mode::DiffWithPreviousConfirmed => {
                let Some(first) = self.load_new()? else {
                    return Ok(None);
                };
                let Some(second) = self.load_previous_confirmed()? else {
                    return Ok(None);
                };
                Ok(Some(pixmap_diff(&first, &second)))
            }
        }
    }

    pub fn unconfirmed_count(&self) -> usize {
        self.all_snapshots
            .iter()
            .flat_map(|s| s.values())
            .filter(|s| s.unconfirmed.is_some())
            .count()
    }

    pub fn current_test_case_index(&self) -> Option<usize> {
        self.current_test_case_index
    }

    pub fn num_test_cases(&self) -> usize {
        self.test_cases.len()
    }

    pub fn num_current_snapshots(&self) -> usize {
        self.current_test_case_index
            .and_then(|index| self.all_snapshots.get(index))
            .map(|snapshots| snapshots.len())
            .unwrap_or(0)
    }

    pub fn has_unconfirmed(&self) -> bool {
        self.current_snapshot()
            .is_some_and(|f| f.unconfirmed.is_some())
    }

    pub fn is_mode_allowed(&self, mode: Mode) -> bool {
        let current_files = self.current_snapshot();
        let has_new = current_files
            .as_ref()
            .is_some_and(|f| f.unconfirmed.is_some());
        let has_confirmed = current_files
            .as_ref()
            .is_some_and(|f| f.confirmed.is_some());
        let has_previous_confirmed = self
            .previous_snapshot()
            .is_some_and(|f| f.confirmed.is_some());

        match mode {
            Mode::New => has_new,
            Mode::Confirmed => has_confirmed,
            Mode::DiffWithConfirmed => has_new && has_confirmed,
            Mode::DiffWithPreviousConfirmed => has_new && has_previous_confirmed,
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if self.is_mode_allowed(mode) {
            self.mode = mode;
        } else {
            warn!("mode not allowed");
        }
    }

    pub fn approve(&mut self) -> anyhow::Result<()> {
        let test_case = self
            .current_test_case_name()
            .context("no current test case")?;
        let test_case_dir = test_snapshots_dir(&self.snapshots_dir, test_case);
        let current_files = self.current_snapshot_mut()?;
        let unconfirmed = current_files
            .unconfirmed
            .as_ref()
            .context("no unconfirmed file")?;

        if let Some(confirmed) = current_files.confirmed.take() {
            fs_err::remove_file(test_case_dir.join(&confirmed.full_name))?;
        }
        let unsuffixed = unconfirmed
            .full_name
            .strip_suffix(".new.png")
            .context("invalid unconfirmed file name")?;
        let confirmed_name = format!("{unsuffixed}.png");
        fs_err::rename(
            test_case_dir.join(&unconfirmed.full_name),
            test_case_dir.join(&confirmed_name),
        )?;
        current_files.confirmed = Some(SingleSnapshotFile {
            full_name: confirmed_name,
            description: unconfirmed.description.clone(),
        });
        current_files.unconfirmed = None;

        self.go_to_next_unconfirmed_file();
        Ok(())
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn current_snapshot_index(&self) -> Option<u32> {
        self.current_snapshot_index
    }

    pub fn run_test_subject(&self) -> anyhow::Result<()> {
        let test_name = self
            .current_test_case_name()
            .context("no current test case")?;
        let child = Command::new("cargo")
            .args(["run", "--", "run", "--default-scale", test_name])
            .current_dir(&self.tests_dir)
            .spawn()?;
        info!("spawned process with pid: {:?}", child.id());
        Ok(())
    }

    pub fn run_test(&self) -> anyhow::Result<process::Child> {
        let test_name = self
            .current_test_case_name()
            .context("no current test case")?;
        let mut command = if let Some(run_script) = &self.run_script {
            Command::new(run_script)
        } else {
            let mut c = Command::new("cargo");
            c.args(["run", "--"]).current_dir(&self.tests_dir);
            c
        };
        let child = command.args(["test", test_name]).spawn()?;
        Ok(child)
    }

    pub fn refresh(&mut self) -> anyhow::Result<()> {
        let data = query_data(&self.tests_dir)?;
        self.all_snapshots = discover_all_snapshots(&data.test_cases, &data.snapshots_dir);
        self.snapshots_dir = data.snapshots_dir;
        if self
            .current_test_case_index
            .is_some_and(|i| i >= self.test_cases.len())
        {
            self.current_test_case_index = self.test_cases.len().checked_sub(1);
        }
        if let Some(test_index) = self.current_test_case_index {
            if let Some(snapshots) = self.all_snapshots.get(test_index) {
                if self
                    .current_snapshot_index
                    .is_some_and(|i| i >= snapshots.len() as u32)
                {
                    self.current_snapshot_index = snapshots.len().checked_sub(1).map(|i| i as u32);
                }
            } else {
                self.current_snapshot_index = None;
            }
        } else {
            self.current_snapshot_index = None;
        }
        Ok(())
    }
}

fn pixmap_diff(a: &Pixmap, b: &Pixmap) -> Pixmap {
    let mut out = Pixmap::new(max(a.width(), b.width()), max(a.height(), b.height())).unwrap();
    let width = out.width();
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
                    if pixel_a == pixel_b {
                        pixel_a
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

    out
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

fn discover_all_snapshots(
    test_cases: &[String],
    test_cases_dir: &Path,
) -> Vec<BTreeMap<u32, SingleSnapshotFiles>> {
    let mut all_snapshots = Vec::new();
    for test_case in test_cases {
        all_snapshots.push(
            discover_snapshots(&test_snapshots_dir(test_cases_dir, test_case)).unwrap_or_else(
                |err| {
                    // TODO: ui message
                    warn!("failed to fetch snapshots: {:?}", err);
                    Default::default()
                },
            ),
        );
    }
    all_snapshots
}
