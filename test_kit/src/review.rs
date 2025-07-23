use {
    crate::{
        discover_snapshots, test_snapshots_dir, Registry, SingleSnapshotFile, SingleSnapshotFiles,
    },
    anyhow::Context,
    log::warn,
    std::{
        cmp::max,
        collections::{BTreeMap, HashMap},
        path::{Path, PathBuf},
        rc::Rc,
    },
    strum::{EnumIter, IntoEnumIterator},
    widgem::{
        event::Event,
        impl_widget_base,
        layout::Layout,
        system::ReportError,
        tiny_skia::{Pixmap, PremultipliedColorU8},
        types::Point,
        widgets::{
            Button, Image, Label, NewWidget, Row, Widget, WidgetBaseOf, WidgetExt, WidgetId, Window,
        },
    },
};

pub struct ReviewWidget {
    base: WidgetBaseOf<Self>,
    test_name_id: WidgetId<Label>,
    snapshot_name_id: WidgetId<Label>,
    coords_id: WidgetId<Label>,
    image_id: WidgetId<Image>,
    approve_and_skip_id: WidgetId<Row>,
    unconfirmed_count_id: WidgetId<Label>,
    // TODO: remove Option
    reviewer: Option<Reviewer>,
    mode_button_ids: HashMap<Mode, WidgetId<Button>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum Mode {
    New,
    Confirmed,
    DiffWithConfirmed,
    DiffWithPreviousConfirmed,
}

impl Mode {
    fn ui_name(self) -> &'static str {
        match self {
            Mode::New => "New",
            Mode::Confirmed => "Confirmed",
            Mode::DiffWithConfirmed => "Diff with confirmed",
            Mode::DiffWithPreviousConfirmed => "Diff with previous confirmed",
        }
    }
}

impl ReviewWidget {
    pub fn set_reviewer(&mut self, reviewer: Reviewer) -> anyhow::Result<()> {
        self.reviewer = Some(reviewer);
        self.update_ui()
    }

    fn update_ui(&mut self) -> anyhow::Result<()> {
        let state = self.reviewer.as_mut().unwrap().current_state();
        self.base
            .widget(self.test_name_id)?
            .set_text(state.test_case_name);
        self.base
            .widget(self.snapshot_name_id)?
            .set_text(state.snapshot_name);
        self.base.widget(self.image_id)?.set_pixmap(state.snapshot);
        for (mode, id) in &self.mode_button_ids {
            self.base
                .widget(*id)?
                .set_enabled(self.reviewer.as_mut().unwrap().is_mode_allowed(*mode));
        }
        self.base
            .widget(self.approve_and_skip_id)?
            .set_enabled(self.reviewer.as_mut().unwrap().has_unconfirmed());
        self.base
            .widget(self.unconfirmed_count_id)?
            .set_text(if state.unconfirmed_count > 0 {
                format!(
                    "Unconfirmed snapshots remaining: {}",
                    state.unconfirmed_count
                )
            } else {
                "No unconfirmed snapshots.".into()
            });
        Ok(())
    }

    fn set_mode(&mut self, mode: Mode) -> anyhow::Result<()> {
        self.reviewer.as_mut().unwrap().set_mode(mode);
        self.update_ui()
    }

    fn image_mouse_move(&mut self, pos_in_widget: Option<Point>) -> anyhow::Result<()> {
        let Some(pos_in_widget) = pos_in_widget else {
            self.base.widget(self.coords_id)?.set_text("");
            return Ok(());
        };
        let pos_in_content = self
            .base
            .widget(self.image_id)?
            .map_widget_pos_to_content_pos(pos_in_widget);
        self.base
            .widget(self.coords_id)?
            .set_text(format!("{:?}", pos_in_content));
        Ok(())
    }
}

impl NewWidget for ReviewWidget {
    type Arg = Reviewer;

    #[allow(clippy::collapsible_if)]
    fn new(mut base: WidgetBaseOf<Self>, arg: Self::Arg) -> Self {
        let id = base.id();
        // TODO: Grid widget

        let window = base.add_child::<Window>("widgem test review".into());
        // TODO: replace with implicit layout
        window.set_layout(Layout::ExplicitGrid);
        let mut current_row = 1;
        window
            .base_mut()
            .add_child::<Label>("Test:".into())
            .set_column(1)
            .set_row(current_row);
        let test_name_id = window
            .base_mut()
            .add_child::<Label>("".into())
            .set_column(2)
            .set_row(current_row)
            .id();
        current_row += 1;

        let row = window
            .base_mut()
            .add_child::<Row>(())
            .set_column(2)
            .set_row(current_row)
            .add_class("no_padding".into())
            .set_layout(Layout::HorizontalFirst);
        current_row += 1;

        row.base_mut()
            .add_child::<Button>("First test".into())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.as_mut().unwrap().go_to_test_case(0);
                w.update_ui()
            }));

        row.base_mut()
            .add_child::<Button>("Previous test".into())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.as_mut().unwrap().go_to_previous_test_case();
                w.update_ui()
            }));
        row.base_mut()
            .add_child::<Button>("Next test".into())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.as_mut().unwrap().go_to_next_test_case();
                w.update_ui()
            }));
        row.base_mut()
            .add_child::<Button>("Last test".into())
            .on_triggered(id.callback(move |w, _e| {
                let index = w
                    .reviewer
                    .as_mut()
                    .unwrap()
                    .test_cases()
                    .len()
                    .saturating_sub(1);
                w.reviewer.as_mut().unwrap().go_to_test_case(index);
                w.update_ui()
            }));

        window
            .base_mut()
            .add_child::<Label>("Snapshot:".into())
            .set_column(1)
            .set_row(current_row);
        let snapshot_name_id = window
            .base_mut()
            .add_child::<Label>("".into())
            .set_column(2)
            .set_row(current_row)
            .id();
        current_row += 1;

        let row = window
            .base_mut()
            .add_child::<Row>(())
            .set_column(2)
            .set_row(current_row)
            .add_class("no_padding".into())
            .set_layout(Layout::HorizontalFirst);
        current_row += 1;

        row.base_mut()
            .add_child::<Button>("Previous snapshot".into())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.as_mut().unwrap().go_to_previous_snapshot();
                w.update_ui()
            }));
        row.base_mut()
            .add_child::<Button>("Next snapshot".into())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.as_mut().unwrap().go_to_next_snapshot();
                w.update_ui()
            }));

        window
            .base_mut()
            .add_child::<Label>("Display mode:".into())
            .set_column(1)
            .set_row(current_row);
        current_row += 1;

        // TODO: radio buttons
        let modes_row = window
            .base_mut()
            .add_child::<Row>(())
            .set_column(2)
            .set_row(current_row)
            .add_class("no_padding".into())
            .set_layout(Layout::HorizontalFirst);
        current_row += 1;

        let mut mode_button_ids = HashMap::new();
        for mode in Mode::iter() {
            // TODO: radio buttons
            let button = modes_row
                .base_mut()
                .add_child::<Button>(mode.ui_name().into())
                .on_triggered(id.callback(move |w, _e| w.set_mode(mode)));
            mode_button_ids.insert(mode, button.id());
        }

        window
            .base_mut()
            .add_child::<Label>("Snapshot:".into())
            .set_column(1)
            .set_row(current_row);

        let row = window
            .base_mut()
            .add_child::<Row>(())
            .set_column(2)
            .set_row(current_row)
            .add_class("no_padding".into())
            .set_layout(Layout::HorizontalFirst);
        current_row += 1;

        row.base_mut()
            .add_child::<Button>("100%".into())
            .on_triggered(id.callback(move |w, _e| {
                w.base.widget(w.image_id)?.set_scale(Some(1.0));
                Ok(())
            }));
        row.base_mut()
            .add_child::<Button>("200%".into())
            .on_triggered(id.callback(move |w, _e| {
                w.base.widget(w.image_id)?.set_scale(Some(2.0));
                Ok(())
            }));
        let coords_id = row.base_mut().add_child::<Label>("".into()).id();
        let image = window
            .base_mut()
            .add_child::<Image>(None)
            .set_column(2)
            .set_row(current_row);
        current_row += 1;

        let image_mouse_move = id.callback(Self::image_mouse_move);
        image.base_mut().event_filter = Some(Box::new(move |event| {
            match event {
                Event::MouseMove(event) => {
                    image_mouse_move.invoke(Some(event.pos));
                }
                Event::MouseLeave(_) => {
                    image_mouse_move.invoke(None);
                }
                _ => (),
            }
            Ok(false)
        }));
        let image_id = image.id();

        window
            .base_mut()
            .add_child::<Label>("Actions:".into())
            .set_column(1)
            .set_row(current_row);

        let approve_and_skip = window
            .base_mut()
            .add_child::<Row>(())
            .set_column(2)
            .set_row(current_row)
            .add_class("no_padding".into())
            .set_layout(Layout::HorizontalFirst);
        current_row += 1;

        approve_and_skip
            .base_mut()
            .add_child::<Button>("Approve".into())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.as_mut().unwrap().approve()?;
                w.update_ui()
            }));
        approve_and_skip
            .base_mut()
            .add_child::<Button>("Skip snapshot".into())
            .on_triggered(id.callback(move |w, _e| {
                if !w.reviewer.as_mut().unwrap().go_to_next_unconfirmed_file() {
                    widgem::exit();
                }
                w.update_ui()
            }));
        approve_and_skip
            .base_mut()
            .add_child::<Button>("Skip test".into())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.as_mut().unwrap().go_to_next_test_case();
                if !w.reviewer.as_mut().unwrap().has_unconfirmed() {
                    if !w.reviewer.as_mut().unwrap().go_to_next_unconfirmed_file() {
                        widgem::exit();
                    }
                }
                w.update_ui()
            }));
        let approve_and_skip_id = approve_and_skip.id();

        let unconfirmed_count_id = window
            .base_mut()
            .add_child::<Label>("".into())
            .set_column(2)
            .set_row(current_row)
            .id();

        let mut w = Self {
            base,
            test_name_id,
            snapshot_name_id,
            image_id,
            coords_id,
            approve_and_skip_id,
            unconfirmed_count_id,
            mode_button_ids,
            reviewer: None,
        };
        w.set_reviewer(arg).or_report_err();
        w
    }

    fn handle_declared(&mut self, arg: Self::Arg) {
        self.set_reviewer(arg).or_report_err();
    }
}

impl Widget for ReviewWidget {
    impl_widget_base!();
}

pub struct Reviewer {
    test_cases_dir: PathBuf,
    mode: Mode,
    test_cases: Vec<String>,
    current_test_case_index: Option<usize>,
    all_snapshots: Vec<BTreeMap<u32, SingleSnapshotFiles>>,
    current_snapshot_index: Option<u32>,
}

impl Reviewer {
    pub fn new(registry: &Registry, test_cases_dir: &Path) -> Self {
        let test_cases: Vec<_> = registry.tests().map(|s| s.to_owned()).collect();
        let mut all_snapshots = Vec::new();
        for test_case in &test_cases {
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
        let mut this = Self {
            test_cases_dir: test_cases_dir.into(),
            mode: Mode::New,
            test_cases,
            current_test_case_index: None,
            all_snapshots,
            current_snapshot_index: None,
        };
        this.go_to_next_test_case();
        this
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
                .is_ok_and(|f| f.unconfirmed.is_some())
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

    fn current_test_case(&self) -> anyhow::Result<&str> {
        self.test_cases
            .get(
                self.current_test_case_index
                    .context("no current test case")?,
            )
            .context("test case not found")
            .map(|s| s.as_str())
    }

    fn previous_snapshot(&self) -> anyhow::Result<&SingleSnapshotFiles> {
        let index = self
            .current_snapshot_index
            .context("no current files")?
            .checked_sub(1)
            .context("no previous files")?;
        self.all_snapshots
            .get(
                self.current_test_case_index
                    .context("no current_test_case_index")?,
            )
            .context("invalid current_test_case_index")?
            .get(&index)
            .context("previous files not found")
    }

    fn current_snapshot(&self) -> anyhow::Result<&SingleSnapshotFiles> {
        self.all_snapshots
            .get(
                self.current_test_case_index
                    .context("no current_test_case_index")?,
            )
            .context("invalid current_test_case_index")?
            .get(&self.current_snapshot_index.context("no current files")?)
            .context("current files not found")
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

    fn load_new(&self) -> anyhow::Result<Pixmap> {
        let path = test_snapshots_dir(&self.test_cases_dir, self.current_test_case()?).join(
            &self
                .current_snapshot()?
                .unconfirmed
                .as_ref()
                .context("no unconfirmed file")?
                .full_name,
        );
        Ok(Pixmap::load_png(path)?)
    }

    fn load_confirmed(&self) -> anyhow::Result<Pixmap> {
        let path = test_snapshots_dir(&self.test_cases_dir, self.current_test_case()?).join(
            &self
                .current_snapshot()?
                .confirmed
                .as_ref()
                .context("no unconfirmed file")?
                .full_name,
        );
        Ok(Pixmap::load_png(path)?)
    }

    fn load_previous_confirmed(&self) -> anyhow::Result<Pixmap> {
        let path = test_snapshots_dir(&self.test_cases_dir, self.current_test_case()?).join(
            &self
                .previous_snapshot()?
                .confirmed
                .as_ref()
                .context("no unconfirmed file")?
                .full_name,
        );
        Ok(Pixmap::load_png(path)?)
    }

    fn make_pixmap(&self) -> anyhow::Result<Pixmap> {
        match self.mode {
            Mode::New => self.load_new(),
            Mode::Confirmed => self.load_confirmed(),
            Mode::DiffWithConfirmed => Ok(pixmap_diff(&self.load_new()?, &self.load_confirmed()?)),
            Mode::DiffWithPreviousConfirmed => Ok(pixmap_diff(
                &self.load_new()?,
                &self.load_previous_confirmed()?,
            )),
        }
    }

    fn current_state(&self) -> ReviewerState {
        let unconfirmed_count = self
            .all_snapshots
            .iter()
            .flat_map(|s| s.values())
            .filter(|s| s.unconfirmed.is_some())
            .count();
        let Ok(test_case) = self.current_test_case() else {
            return ReviewerState {
                test_case_name: "none".into(),
                snapshot_name: "none".into(),
                mode: Mode::Confirmed,
                snapshot: None,
                unconfirmed_count,
            };
        };
        let test_case_name = format!(
            "({}/{}) {:?}",
            self.current_test_case_index.unwrap() + 1,
            self.test_cases.len(),
            test_case
        );
        let Ok(current_files) = self.current_snapshot() else {
            return ReviewerState {
                test_case_name,
                snapshot_name: "none".into(),
                mode: Mode::Confirmed,
                snapshot: None,
                unconfirmed_count,
            };
        };
        let snapshot_name = match self.mode {
            Mode::New | Mode::DiffWithConfirmed | Mode::DiffWithPreviousConfirmed => current_files
                .unconfirmed
                .as_ref()
                .map_or_else(|| "none".into(), |f| f.description.clone()),
            Mode::Confirmed => current_files
                .confirmed
                .as_ref()
                .map_or_else(|| "none".into(), |f| f.description.clone()),
        };
        let Some(snapshots) = self
            .current_test_case_index
            .and_then(|index| self.all_snapshots.get(index))
        else {
            warn!("invalid current_test_case_index2");
            return ReviewerState {
                test_case_name,
                snapshot_name: "none".into(),
                mode: Mode::Confirmed,
                snapshot: None,
                unconfirmed_count,
            };
        };
        let snapshot_name = format!(
            "({}/{}) {}",
            self.current_snapshot_index.unwrap(),
            snapshots.len(),
            snapshot_name
        );

        ReviewerState {
            test_case_name,
            mode: self.mode,
            snapshot_name,
            snapshot: self
                .make_pixmap()
                .map_err(|err| {
                    warn!("failed to make pixmap: {:?}", err);
                })
                .ok()
                .map(Rc::new),
            unconfirmed_count,
        }
    }

    pub fn has_unconfirmed(&self) -> bool {
        let current_files = self.current_snapshot();
        current_files.is_ok_and(|f| f.unconfirmed.is_some())
    }

    pub fn is_mode_allowed(&self, mode: Mode) -> bool {
        let current_files = self.current_snapshot();
        let has_new = current_files
            .as_ref()
            .is_ok_and(|f| f.unconfirmed.is_some());
        let has_confirmed = current_files.as_ref().is_ok_and(|f| f.confirmed.is_some());
        let has_previous_confirmed = self
            .previous_snapshot()
            .is_ok_and(|f| f.confirmed.is_some());

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
        let test_case = self.current_test_case()?;
        let test_case_dir = test_snapshots_dir(&self.test_cases_dir, test_case);
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
}

struct ReviewerState {
    test_case_name: String,
    snapshot_name: String,
    // TODO: use it to set active radio button
    #[allow(dead_code)]
    mode: Mode,
    snapshot: Option<Rc<Pixmap>>,
    unconfirmed_count: usize,
}

fn pixmap_diff(a: &Pixmap, b: &Pixmap) -> Pixmap {
    let mut out = Pixmap::new(max(a.width(), b.width()), max(a.height(), b.height())).unwrap();
    let width = out.width();
    for y in 0..out.height() {
        for x in 0..width {
            let pixel_a = a.pixel(x, y);
            let pixel_b = b.pixel(x, y);
            let pixel_out = if pixel_a == pixel_b {
                pixel_a.unwrap_or_else(|| PremultipliedColorU8::from_rgba(255, 0, 0, 255).unwrap())
            // } else if let (Some(pixel_a), Some(pixel_b)) = (pixel_a, pixel_b) {
            //     PremultipliedColorU8::from_rgba(
            //         u8_diff(pixel_a.red(), pixel_b.red()),
            //         u8_diff(pixel_a.green(), pixel_b.green()),
            //         u8_diff(pixel_a.blue(), pixel_b.blue()),
            //         255,
            //     )
            //     .unwrap()
            } else if let Some(pixel_a) = pixel_a {
                PremultipliedColorU8::from_rgba(
                    pixel_a.red().saturating_sub(50),
                    pixel_a.green().saturating_add(50),
                    pixel_a.blue().saturating_sub(50),
                    255,
                )
                .unwrap()
            } else {
                PremultipliedColorU8::from_rgba(255, 0, 0, 255).unwrap()
            };
            out.pixels_mut()[(y * width + x) as usize] = pixel_out;
        }
    }

    out
}
