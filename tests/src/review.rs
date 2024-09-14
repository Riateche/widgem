use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
};

use anyhow::Context;
use log::warn;
use salvation::{
    impl_widget_common,
    layout::LayoutItemOptions,
    tiny_skia::Pixmap,
    widgets::{
        button::Button, image::Image, label::Label, row::Row, Widget, WidgetCommon, WidgetExt,
        WidgetId,
    },
    WindowAttributes,
};
use strum::{EnumIter, IntoEnumIterator};

use crate::{discover_snapshots, test_cases::TestCase, test_cases_dir, SingleSnapshotFiles};

pub struct ReviewWidget {
    common: WidgetCommon,
    test_name_id: WidgetId<Label>,
    snapshot_name_id: WidgetId<Label>,
    image_id: WidgetId<Image>,
    reviewer: Reviewer,
}

#[derive(Debug, Clone, Copy, EnumIter)]
pub enum Mode {
    New,
    Confirmed,
    PreviousConfirmed,
    DiffWithConfirmed,
    DiffWithPreviousConfirmed,
}

impl Mode {
    fn ui_name(self) -> &'static str {
        match self {
            Mode::New => "New",
            Mode::Confirmed => "Confirmed",
            Mode::PreviousConfirmed => "Previous confirmed",
            Mode::DiffWithConfirmed => "Diff with confirmed",
            Mode::DiffWithPreviousConfirmed => "Diff with previous confirmed",
        }
    }
}

impl ReviewWidget {
    pub fn new(reviewer: Reviewer) -> Self {
        let mut common = WidgetCommon::new();
        common.add_child(
            Label::new("Test:").boxed(),
            LayoutItemOptions::from_pos_in_grid(1, 1),
        );
        let test_name = Label::new("").with_id();
        common.add_child(
            test_name.widget.boxed(),
            LayoutItemOptions::from_pos_in_grid(2, 1),
        );

        common.add_child(
            Label::new("Snapshot:").boxed(),
            LayoutItemOptions::from_pos_in_grid(1, 2),
        );
        let snapshot_name = Label::new("").with_id();
        common.add_child(
            snapshot_name.widget.boxed(),
            LayoutItemOptions::from_pos_in_grid(2, 2),
        );

        common.add_child(
            Label::new("Display mode:").boxed(),
            LayoutItemOptions::from_pos_in_grid(1, 3),
        );

        let mut modes_row = Row::new();
        for mode in Mode::iter() {
            modes_row.add_child(
                Button::new(mode.ui_name())
                    .with_on_triggered(common.id.callback(move |w: &mut Self, _e| w.set_mode(mode)))
                    .boxed(),
            )
        }
        // TODO: radio buttons
        common.add_child(
            // Row::new()
            // .with_child(Button::new("New").boxed())
            // .with_child(Button::new("Confirmed").boxed())
            // .with_child(Button::new("Previous confirmed").boxed())
            // .with_child(Button::new("Diff with confirmed").boxed())
            // .with_child(Button::new("Diff with previous confirmed").boxed())
            modes_row.boxed(),
            LayoutItemOptions::from_pos_in_grid(2, 3),
        );

        common.add_child(
            Label::new("Snapshot:").boxed(),
            LayoutItemOptions::from_pos_in_grid(1, 4),
        );
        let image = Image::new(None).with_id();
        common.add_child(
            image.widget.boxed(),
            LayoutItemOptions::from_pos_in_grid(2, 4),
        );

        common.add_child(
            Label::new("Actions:").boxed(),
            LayoutItemOptions::from_pos_in_grid(1, 5),
        );
        common.add_child(
            Row::new()
                .with_child(Button::new("Approve").boxed())
                .with_child(Button::new("Skip snapshot").boxed())
                .with_child(Button::new("Skip test").boxed())
                .boxed(),
            LayoutItemOptions::from_pos_in_grid(2, 5),
        );

        let mut this = Self {
            common,
            test_name_id: test_name.id,
            snapshot_name_id: snapshot_name.id,
            image_id: image.id,
            reviewer,
        }
        .with_window(WindowAttributes::default().with_title("salvation test review"));
        this.update_ui().unwrap();
        this
    }

    fn update_ui(&mut self) -> anyhow::Result<()> {
        let state = self.reviewer.current_state();
        self.common
            .widget(self.test_name_id)?
            .set_text(state.test_case_name);
        self.common
            .widget(self.snapshot_name_id)?
            .set_text(state.snapshot_name);
        self.common
            .widget(self.image_id)?
            .set_pixmap(state.snapshot);

        Ok(())
    }

    fn set_mode(&mut self, mode: Mode) -> anyhow::Result<()> {
        self.reviewer.set_mode(mode);
        self.update_ui()
    }
}

impl Widget for ReviewWidget {
    impl_widget_common!();
}

pub struct Reviewer {
    test_cases_dir: PathBuf,
    mode: Mode,
    remaining_test_cases: VecDeque<TestCase>,
    current_test_case: Option<TestCase>,
    remaining_files: BTreeMap<u32, SingleSnapshotFiles>,
    previous_files: Option<SingleSnapshotFiles>,
    current_files: Option<SingleSnapshotFiles>,
}

impl Reviewer {
    pub fn new(test_cases_dir: &Path) -> Self {
        let mut this = Self {
            test_cases_dir: test_cases_dir.into(),
            mode: Mode::New,
            remaining_test_cases: TestCase::iter().collect(),
            current_test_case: None,
            remaining_files: Default::default(),
            previous_files: None,
            current_files: None,
        };
        this.go_to_next_test_case();
        this
    }

    fn go_to_next_test_case(&mut self) {
        loop {
            self.current_test_case = None;
            self.remaining_files.clear();
            self.current_files = None;
            self.previous_files = None;
            let Some(test_case) = self.remaining_test_cases.pop_front() else {
                return;
            };
            self.current_test_case = Some(test_case);
            self.remaining_files =
                discover_snapshots(&self.test_cases_dir.join(format!("{:?}", test_case)))
                    .unwrap_or_else(|err| {
                        // TODO: ui message
                        warn!("failed to fetch snapshots: {:?}", err);
                        Default::default()
                    });
            self.go_to_next_files();
            if self.current_files.is_some() {
                return;
            }
        }
    }

    fn go_to_next_files(&mut self) {
        self.current_files = None;
        self.mode = Mode::New;
        while let Some((_, files)) = self.remaining_files.pop_first() {
            self.previous_files = self.current_files.take();
            let has_unconfirmed = files.unconfirmed.is_some();
            self.current_files = Some(files);
            if has_unconfirmed {
                return;
            }
        }
        self.previous_files = None;
        self.current_files = None;
    }

    fn load_new(&self) -> anyhow::Result<Pixmap> {
        Pixmap::load_png(self.test_cases_dir.join(format!(
            "{:?}/{}",
            self.current_test_case.unwrap(),
            self.current_files.as_ref().context("no current files")?.unconfirmed.clone().unwrap()
        )))
        .map_err(From::from)
    }

    fn load_confirmed(&self) -> anyhow::Result<Pixmap> {
        Pixmap::load_png(self.test_cases_dir.join(format!(
                "{:?}/{}",
                self.current_test_case.unwrap(),
                self.current_files
                    .as_ref()
                    .context("no current files")?
                    .confirmed
                    .clone()
                    .context("no confirmed snapshot")?
            )))
        .map_err(From::from)
    }

    fn current_state(&self) -> ReviewerState {
        let test_case_name = self
            .current_test_case
            .map(|t| format!("{:?}", t))
            .unwrap_or_else(|| "none".into());
        let snapshot_name: String;
        let snapshot: Option<Pixmap>;
        if let Some(current_files) = &self.current_files {
            // TODO: name should depend on mode

            match self.mode {
                Mode::New => {
                    snapshot_name = current_files.unconfirmed.clone().unwrap();
                    snapshot = self
                        .load_new()
                        .map_err(|err| {
                            warn!("failed to load png: {:?}", err);
                        })
                        .ok();
                }
                Mode::Confirmed => {
                    snapshot_name = current_files.confirmed.clone().unwrap();
                    snapshot = self
                        .load_confirmed()
                        .map_err(|err| {
                            warn!("failed to load png: {:?}", err);
                        })
                        .ok();
                }
                Mode::PreviousConfirmed => todo!(),
                Mode::DiffWithConfirmed => todo!(),
                Mode::DiffWithPreviousConfirmed => todo!(),
            }
        } else {
            snapshot_name = "none".into();
            snapshot = None;
        };

        ReviewerState {
            test_case_name,
            mode: self.mode,
            snapshot_name,
            snapshot,
        }
    }

    pub fn has_current_files(&self) -> bool {
        self.current_files.is_some()
    }

    pub fn is_mode_allowed(&self, mode: Mode) -> bool {
        match mode {
            Mode::New => self.current_files.is_some(),
            Mode::Confirmed | Mode::DiffWithConfirmed => self
                .current_files
                .as_ref()
                .map_or(false, |f| f.confirmed.is_some()),
            Mode::DiffWithPreviousConfirmed | Mode::PreviousConfirmed => {
                self.current_files.is_some()
                    && self
                        .previous_files
                        .as_ref()
                        .map_or(false, |f| f.confirmed.is_some())
            }
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if self.is_mode_allowed(mode) {
            self.mode = mode;
        } else {
            warn!("mode not allowed");
        }
    }
}

struct ReviewerState {
    test_case_name: String,
    snapshot_name: String,
    mode: Mode,
    snapshot: Option<Pixmap>,
}
