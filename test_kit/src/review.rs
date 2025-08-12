use {
    crate::{
        discover_snapshots, test_snapshots_dir, Registry, SingleSnapshotFile, SingleSnapshotFiles,
    },
    anyhow::Context,
    log::warn,
    std::{
        cmp::max,
        collections::BTreeMap,
        path::{Path, PathBuf},
    },
    strum::{EnumIter, IntoEnumIterator},
    tiny_skia::{Pixmap, PremultipliedColorU8},
    widgem::{
        event::Event,
        impl_widget_base,
        layout::Layout,
        system::ReportError,
        types::Point,
        widgets::{
            Button, Column, Image, Label, NewWidget, Row, ScrollArea, Widget, WidgetBaseOf,
            WidgetExt, WidgetId, Window,
        },
    },
};

pub struct ReviewWidget {
    base: WidgetBaseOf<Self>,
    reviewer: Reviewer,
    coords: String,
    image_scale: f32,
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
        self.reviewer = reviewer;
        self.base.update();
        Ok(())
    }

    fn set_mode(&mut self, mode: Mode) -> anyhow::Result<()> {
        self.reviewer.set_mode(mode);
        self.base.update();
        Ok(())
    }

    fn image_mouse_move(
        &mut self,
        (image_id, pos_in_widget): (WidgetId<Image>, Option<Point>),
    ) -> anyhow::Result<()> {
        let Some(pos_in_widget) = pos_in_widget else {
            self.coords.clear();
            self.base.update();
            return Ok(());
        };
        let pos_in_content = self
            .base
            .find_child_mut(image_id)?
            .map_widget_pos_to_content_pos(pos_in_widget);
        self.coords = format!(
            "X: {}; Y: {}",
            pos_in_content.x().to_i32(),
            pos_in_content.y().to_i32()
        );
        self.base.update();
        Ok(())
    }
}

impl NewWidget for ReviewWidget {
    type Arg = Reviewer;

    #[allow(clippy::collapsible_if)]
    fn new(base: WidgetBaseOf<Self>, reviewer: Self::Arg) -> Self {
        Self {
            base,
            reviewer,
            coords: String::new(),
            image_scale: 1.0,
        }
    }

    fn handle_declared(&mut self, arg: Self::Arg) {
        self.set_reviewer(arg).or_report_err();
    }
}

impl Widget for ReviewWidget {
    impl_widget_base!();

    fn handle_declare_children_request(&mut self) -> anyhow::Result<()> {
        let id = self.base.id();

        let window = self
            .base
            .declare_child::<Window>("widgem snapshot review".into());
        window.set_layout(Layout::ExplicitGrid);
        let mut current_row = 1;
        window
            .base_mut()
            .declare_child::<Label>("Test:".into())
            .set_grid_cell(1, current_row);
        let test_case_name = self
            .reviewer
            .current_test_case_name()
            .and_then(|name| {
                Some(format!(
                    "({}/{}) {:?}",
                    self.reviewer.current_test_case_index()? + 1,
                    self.reviewer.num_test_cases(),
                    name
                ))
            })
            .unwrap_or_else(|| "none".into());
        window
            .base_mut()
            .declare_child::<Label>(test_case_name)
            .set_grid_cell(2, current_row);
        current_row += 1;

        let row = window
            .base_mut()
            .declare_child::<Row>(())
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false);
        current_row += 1;

        row.base_mut()
            .declare_child::<Button>("First test".into())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.go_to_test_case(0);
                w.base.update();
                Ok(())
            }));

        row.base_mut()
            .declare_child::<Button>("Previous test".into())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.go_to_previous_test_case();
                w.base.update();
                Ok(())
            }));
        row.base_mut()
            .declare_child::<Button>("Next test".into())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.go_to_next_test_case();
                w.base.update();
                Ok(())
            }));
        row.base_mut()
            .declare_child::<Button>("Last test".into())
            .on_triggered(id.callback(move |w, _e| {
                let index = w.reviewer.test_cases().len().saturating_sub(1);
                w.reviewer.go_to_test_case(index);
                w.base.update();
                Ok(())
            }));

        window
            .base_mut()
            .declare_child::<Label>("Snapshot:".into())
            .set_grid_cell(1, current_row);

        let snapshot_name = self
            .reviewer
            .current_snapshot()
            .and_then(|current_files| match self.reviewer.mode {
                Mode::New | Mode::DiffWithConfirmed | Mode::DiffWithPreviousConfirmed => {
                    current_files
                        .unconfirmed
                        .as_ref()
                        .map(|f| f.description.clone())
                }
                Mode::Confirmed => current_files
                    .confirmed
                    .as_ref()
                    .map(|f| f.description.clone()),
            })
            .and_then(|description| {
                let index = self.reviewer.current_snapshot_index?;
                Some(format!(
                    "({}/{}) {:?}",
                    index,
                    self.reviewer.num_current_snapshots(),
                    description
                ))
            })
            .unwrap_or_else(|| "none".into());
        window
            .base_mut()
            .declare_child::<Label>(snapshot_name)
            .set_grid_cell(2, current_row);
        current_row += 1;

        let row = window
            .base_mut()
            .declare_child::<Row>(())
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false);
        current_row += 1;

        row.base_mut()
            .declare_child::<Button>("Previous snapshot".into())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.go_to_previous_snapshot();
                w.base.update();
                Ok(())
            }));
        row.base_mut()
            .declare_child::<Button>("Next snapshot".into())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.go_to_next_snapshot();
                w.base.update();
                Ok(())
            }));

        window
            .base_mut()
            .declare_child::<Label>("Display mode:".into())
            .set_grid_cell(1, current_row);
        current_row += 1;

        // TODO: radio buttons
        let modes_row = window
            .base_mut()
            .declare_child::<Row>(())
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false);
        current_row += 1;

        for mode in Mode::iter() {
            // TODO: radio buttons
            modes_row
                .base_mut()
                .declare_child::<Button>(mode.ui_name().into())
                .set_enabled(self.reviewer.is_mode_allowed(mode))
                .on_triggered(id.callback(move |w, _e| w.set_mode(mode)));
        }

        window
            .base_mut()
            .declare_child::<Label>("Snapshot:".into())
            .set_grid_cell(1, current_row);

        let row = window
            .base_mut()
            .declare_child::<Row>(())
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false);
        current_row += 1;

        row.base_mut()
            .declare_child::<Button>("100%".into())
            .on_triggered(id.callback(move |w, _e| {
                w.image_scale = 1.0;
                w.base.update();
                Ok(())
            }));
        row.base_mut()
            .declare_child::<Button>("200%".into())
            .on_triggered(id.callback(move |w, _e| {
                w.image_scale = 2.0;
                w.base.update();
                Ok(())
            }));
        row.base_mut()
            .declare_child::<Button>("400%".into())
            .on_triggered(id.callback(move |w, _e| {
                w.image_scale = 4.0;
                w.base.update();
                Ok(())
            }));
        row.base_mut()
            .declare_child::<Button>("800%".into())
            .on_triggered(id.callback(move |w, _e| {
                w.image_scale = 8.0;
                w.base.update();
                Ok(())
            }));
        row.base_mut().declare_child::<Label>(self.coords.clone());

        let pixmap = self
            .reviewer
            .make_pixmap()
            .or_report_err()
            .flatten()
            .map(Into::into);
        let image = window
            .base_mut()
            .declare_child::<ScrollArea>(())
            .set_grid_cell(2, current_row)
            .set_content::<Column>(())
            .set_style("Column { background: #c0c0c0; padding: 2px; }")
            .base_mut()
            .declare_child::<Image>(pixmap)
            .set_scale(Some(self.image_scale));
        current_row += 1;

        let image_mouse_move = id.callback(Self::image_mouse_move);
        let image_id = image.id();
        image
            .base_mut()
            .install_event_filter(id.raw(), move |event| {
                match event {
                    Event::MouseMove(event) => {
                        image_mouse_move.invoke((image_id, Some(event.pos())));
                    }
                    Event::MouseLeave(_) => {
                        image_mouse_move.invoke((image_id, None));
                    }
                    _ => (),
                }
                Ok(false)
            });

        window
            .base_mut()
            .declare_child::<Label>("Actions:".into())
            .set_grid_cell(1, current_row);

        let approve_and_skip = window
            .base_mut()
            .declare_child::<Row>(())
            .set_grid_cell(2, current_row)
            .set_padding_enabled(false);
        current_row += 1;

        approve_and_skip
            .base_mut()
            .declare_child::<Button>("Approve".into())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.approve()?;
                w.base_mut().update();
                Ok(())
            }));
        approve_and_skip
            .base_mut()
            .declare_child::<Button>("Skip snapshot".into())
            .on_triggered(id.callback(move |w, _e| {
                if !w.reviewer.go_to_next_unconfirmed_file() {
                    widgem::exit();
                }
                w.base.update();
                Ok(())
            }));
        #[allow(clippy::collapsible_if)]
        approve_and_skip
            .base_mut()
            .declare_child::<Button>("Skip test".into())
            .set_enabled(self.reviewer.has_unconfirmed())
            .on_triggered(id.callback(move |w, _e| {
                w.reviewer.go_to_next_test_case();
                if !w.reviewer.has_unconfirmed() {
                    if !w.reviewer.go_to_next_unconfirmed_file() {
                        widgem::exit();
                    }
                }
                w.base.update();
                Ok(())
            }));

        let unconfirmed_count = self.reviewer.unconfirmed_count();
        window
            .base_mut()
            .declare_child::<Label>(if unconfirmed_count > 0 {
                format!("Unconfirmed snapshots remaining: {}", unconfirmed_count)
            } else {
                "No unconfirmed snapshots.".into()
            })
            .set_grid_cell(2, current_row);

        Ok(())
    }
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

    fn current_test_case_name(&self) -> Option<&str> {
        Some(self.test_cases.get(self.current_test_case_index?)?.as_str())
    }

    fn previous_snapshot(&self) -> Option<&SingleSnapshotFiles> {
        let index = self.current_snapshot_index?.checked_sub(1)?;
        self.all_snapshots
            .get(self.current_test_case_index?)?
            .get(&index)
    }

    fn current_snapshot(&self) -> Option<&SingleSnapshotFiles> {
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
        let path = test_snapshots_dir(&self.test_cases_dir, test_case).join(&unconfirmed.full_name);
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
        let path = test_snapshots_dir(&self.test_cases_dir, test_case).join(&confirmed.full_name);
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
        let path = test_snapshots_dir(&self.test_cases_dir, test_case).join(&confirmed.full_name);
        Ok(Some(Pixmap::load_png(path)?))
    }

    fn make_pixmap(&self) -> anyhow::Result<Option<Pixmap>> {
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

    fn unconfirmed_count(&self) -> usize {
        self.all_snapshots
            .iter()
            .flat_map(|s| s.values())
            .filter(|s| s.unconfirmed.is_some())
            .count()
    }

    fn current_test_case_index(&self) -> Option<usize> {
        self.current_test_case_index
    }

    fn num_test_cases(&self) -> usize {
        self.test_cases.len()
    }

    fn num_current_snapshots(&self) -> usize {
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
            } else if let Some(_pixel_a) = pixel_a {
                // PremultipliedColorU8::from_rgba(
                //     pixel_a.red().saturating_sub(50),
                //     pixel_a.green().saturating_add(50),
                //     pixel_a.blue().saturating_sub(50),
                //     255,
                // )
                // .unwrap()
                PremultipliedColorU8::from_rgba(255, 0, 0, 255).unwrap()
            } else {
                PremultipliedColorU8::from_rgba(255, 0, 0, 255).unwrap()
            };
            out.pixels_mut()[(y * width + x) as usize] = pixel_out;
        }
    }

    out
}
