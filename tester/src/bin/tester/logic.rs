use {
    crate::data::{Config, Position, Tests},
    anyhow::Context,
    std::process::{self, Command, Stdio},
    strum::EnumIter,
    tracing::{info, warn},
    widgem::Pixmap,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum Mode {
    New,
    Confirmed,
    DiffWithConfirmed,
    DiffWithPreviousConfirmed,
}

pub struct TesterLogic {
    tests: Tests,
    mode: Mode,
    position: Option<Position>,
}

impl TesterLogic {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let mut this = Self {
            tests: Tests::new(config)?,
            mode: Mode::Confirmed,
            position: None,
        };
        if this.tests.has_unconfirmed_snapshots() {
            this.go_to_next_unconfirmed_snapshot();
            if this.is_mode_allowed(Mode::New) {
                this.set_mode(Mode::New);
            }
        } else {
            this.go_to_next_test_case();
        }
        Ok(this)
    }

    #[allow(clippy::collapsible_if)]
    pub fn go_to_next_unconfirmed_snapshot(&mut self) -> bool {
        if let Some(pos) = self.tests.next_unconfirmed_pos(self.position.as_ref()) {
            self.position = Some(pos);
            self.adjust_mode();
            true
        } else {
            false
        }
    }

    pub fn has_previous_test_case(&self) -> bool {
        self.tests.previous_test(self.position.as_ref()).is_some()
    }

    pub fn has_next_test_case(&self) -> bool {
        self.tests.next_test(self.position.as_ref()).is_some()
    }

    pub fn go_to_next_test_case(&mut self) {
        if let Some(pos) = self.tests.next_test(self.position.as_ref()) {
            self.position = Some(pos);
            self.adjust_mode();
        }
    }

    fn adjust_mode(&mut self) {
        if self.is_mode_allowed(self.mode) {
            return;
        }
        self.mode = if self.has_unconfirmed() {
            Mode::New
        } else {
            Mode::Confirmed
        };
    }

    pub fn go_to_previous_test_case(&mut self) {
        if let Some(pos) = self.tests.previous_test(self.position.as_ref()) {
            self.position = Some(pos);
            self.adjust_mode();
        }
    }

    pub fn go_to_first_test_case(&mut self) {
        if let Some(pos) = self.tests.next_test(None) {
            self.position = Some(pos);
            self.adjust_mode();
        }
    }

    pub fn go_to_last_test_case(&mut self) {
        if let Some(pos) = self.tests.previous_test(None) {
            self.position = Some(pos);
            self.adjust_mode();
        }
    }

    pub fn has_previous_snapshot(&self) -> bool {
        self.position
            .as_ref()
            .is_some_and(|pos| self.tests.previous_snapshot(pos).is_some())
    }

    pub fn has_next_snapshot(&self) -> bool {
        self.position
            .as_ref()
            .is_some_and(|pos| self.tests.next_snapshot(pos).is_some())
    }

    pub fn go_to_previous_snapshot(&mut self) {
        let Some(position) = self.position.as_ref() else {
            return;
        };
        if let Some(pos) = self.tests.previous_snapshot(position) {
            self.position = Some(pos);
            self.adjust_mode();
        }
    }

    pub fn go_to_next_snapshot(&mut self) {
        let Some(position) = self.position.as_ref() else {
            return;
        };
        if let Some(pos) = self.tests.next_snapshot(position) {
            self.position = Some(pos);
            self.adjust_mode();
        }
    }

    pub fn current_test_case_name(&self) -> Option<&str> {
        self.position.as_ref().map(|pos| pos.test.as_str())
    }

    pub fn current_test_case_index(&self) -> Option<usize> {
        self.position
            .as_ref()
            .and_then(|pos| self.tests.test_index(&pos.test))
    }

    pub fn unconfirmed_description(&self) -> Option<&str> {
        self.position
            .as_ref()
            .and_then(|pos| self.tests.unconfirmed_description(pos))
    }

    pub fn confirmed_description(&self) -> Option<&str> {
        self.position
            .as_ref()
            .and_then(|pos| self.tests.confirmed_description(pos))
    }

    pub fn pixmap(&mut self) -> anyhow::Result<Option<Pixmap>> {
        let Some(pos) = self.position.as_ref() else {
            return Ok(None);
        };
        match self.mode {
            Mode::New => self.tests.unconfirmed_pixmap(pos),
            Mode::Confirmed => self.tests.confirmed_pixmap(pos),
            Mode::DiffWithConfirmed => self.tests.diff_with_confirmed(pos),
            Mode::DiffWithPreviousConfirmed => self.tests.diff_with_previous_confirmed(pos),
        }
    }

    pub fn num_current_snapshots(&self) -> usize {
        self.position
            .as_ref()
            .map_or(0, |pos| self.tests.num_snapshots(&pos.test))
    }

    pub fn has_unconfirmed(&self) -> bool {
        self.position
            .as_ref()
            .is_some_and(|pos| self.tests.has_unconfirmed(pos))
    }

    pub fn has_confirmed(&self) -> bool {
        self.position
            .as_ref()
            .is_some_and(|pos| self.tests.has_confirmed(pos))
    }

    pub fn is_mode_allowed(&self, mode: Mode) -> bool {
        let has_new = self.has_unconfirmed();
        let has_confirmed = self.has_confirmed();
        let has_previous_confirmed = self
            .position
            .as_ref()
            .and_then(|pos| self.tests.previous_snapshot(pos))
            .is_some_and(|prev_pos| self.tests.has_confirmed(&prev_pos));

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
        let pos = self.position.as_ref().context("no current snapshot")?;
        self.tests.approve(pos)?;
        if self.tests.has_unconfirmed_snapshots() {
            self.go_to_next_unconfirmed_snapshot();
        }
        self.adjust_mode();
        Ok(())
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn current_snapshot_index(&self) -> Option<u32> {
        self.position.as_ref().and_then(|pos| pos.snapshot)
    }

    pub fn run_test_subject(&self) -> anyhow::Result<()> {
        let test_name = self
            .current_test_case_name()
            .context("no current test case")?;
        let child = Command::new("cargo")
            .args(["run", "--", "run", "--default-scale", test_name])
            .current_dir(&self.tests.config().tests_dir)
            .spawn()?;
        info!("spawned process with pid: {:?}", child.id());
        Ok(())
    }

    pub fn run_test(&self) -> anyhow::Result<process::Child> {
        let test_name = self
            .current_test_case_name()
            .context("no current test case")?;
        let mut command = if let Some(run_script) = &self.tests.config().run_script {
            Command::new(run_script)
        } else {
            let mut c = Command::new("cargo");
            c.args(["run", "--"])
                .current_dir(&self.tests.config().tests_dir);
            c
        };
        let child = command
            .args(["test", test_name])
            .env("NO_COLOR", "1")
            .env("CARGO_TERM_COLOR", "never")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(child)
    }

    pub fn refresh(&mut self) -> anyhow::Result<()> {
        self.tests.refresh()?;
        self.position = self
            .position
            .as_ref()
            .and_then(|pos| self.tests.closest_valid_pos(pos));
        self.adjust_mode();
        Ok(())
    }

    pub fn tests(&self) -> &Tests {
        &self.tests
    }
}
