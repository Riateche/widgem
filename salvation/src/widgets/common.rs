use {
    super::{RawWidgetId, Widget, WidgetAddress, WidgetExt, WidgetId, WidgetNotFound},
    crate::{
        callback::{widget_callback, Callback},
        event::{Event, LayoutEvent},
        layout::{
            grid::{GridAxisOptions, GridOptions},
            Alignment, LayoutItemOptions, SizeHintMode,
        },
        shortcut::{Shortcut, ShortcutId, ShortcutScope},
        style::{
            computed::{CommonComputedStyle, ComputedElementStyle, ComputedStyle},
            css::{Element, MyPseudoClass},
        },
        system::{register_address, unregister_address, with_system},
        types::{Point, Rect, Size},
        window::{Window, WindowId},
    },
    anyhow::{bail, Context, Result},
    derivative::Derivative,
    log::warn,
    std::{
        collections::{btree_map, BTreeMap, HashMap},
        fmt::Debug,
        marker::PhantomData,
        ops::{Deref, DerefMut},
        rc::Rc,
    },
    winit::window::CursorIcon,
};

#[derive(Debug, Clone)]
pub struct WidgetCreationContext {
    pub parent_id: Option<RawWidgetId>,
    pub address: WidgetAddress,
    pub window: Option<Window>,
    pub parent_style: ComputedStyle,
    pub is_parent_enabled: bool,
    pub is_window_root: bool,
}

pub type EventFilterFn = dyn Fn(Event) -> Result<bool>;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Child {
    #[derivative(Debug = "ignore")]
    pub widget: Box<dyn Widget>,
    pub rect_in_parent: Option<Rect>,
    pub rect_set_during_layout: bool,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct WidgetCommon {
    pub id: RawWidgetId,
    pub is_focusable: bool,
    pub enable_ime: bool,
    pub cursor_icon: CursorIcon,
    // If true, all mouse events from the parent propagate to this widget,
    // regardless of its boundaries.
    pub receives_all_mouse_events: bool,

    pub is_focused: bool,
    pub is_window_focused: bool,

    pub parent_id: Option<RawWidgetId>,
    pub address: WidgetAddress,
    pub window: Option<Window>,

    pub parent_style: ComputedStyle,
    pub is_parent_enabled: bool,
    pub is_self_enabled: bool,
    pub is_self_visible: bool,

    pub is_window_root: bool,

    pub is_mouse_over: bool,
    // Present if the widget is mounted, not hidden, and only after layout.
    pub rect_in_window: Option<Rect>,
    // In this widget's coordinates.
    pub visible_rect: Option<Rect>,

    pub children: BTreeMap<Key, Child>,
    pub layout_item_options: LayoutItemOptions,
    pub current_layout_event: Option<LayoutEvent>,

    pub size_hint_x_cache: HashMap<SizeHintMode, i32>,
    // TODO: limit count
    pub size_hint_y_cache: HashMap<(i32, SizeHintMode), i32>,
    pub size_x_fixed_cache: Option<bool>,
    pub size_y_fixed_cache: Option<bool>,

    pub is_accessible: bool,
    pub pending_accessible_update: bool,

    pub self_style: Option<ComputedStyle>,

    pub is_registered_as_focusable: bool,
    // TODO: multiple filters?
    // TODO: accept/reject event from filter; option to run filter after on_event
    #[derivative(Debug = "ignore")]
    pub event_filter: Option<Box<EventFilterFn>>,
    pub grid_options: Option<GridOptions>,
    pub no_padding: bool,

    pub shortcuts: Vec<Shortcut>,
    pub style_element: Element,
    pub common_style: Rc<CommonComputedStyle>,
}

// TODO: enum with other types
pub type Key = u64;

impl Drop for WidgetCommon {
    fn drop(&mut self) {
        unregister_address(self.id);
        // Drop and unmount children before unmounting self.
        self.children.clear();
        self.unmount_accessible();
        for shortcut in &self.shortcuts {
            // TODO: deregister widget/window shortcuts
            if shortcut.scope == ShortcutScope::Application {
                with_system(|system| system.application_shortcuts.retain(|s| s.id != shortcut.id));
            }
        }
    }
}

impl WidgetCommon {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<T: Widget>(ctx: WidgetCreationContext) -> WidgetCommonTyped<T> {
        let id = ctx.address.widget_id();
        register_address(id, ctx.address.clone());

        let style_element = Element::new(T::type_name());
        let common_style = ctx.parent_style.get_common(&style_element);
        let mut common = Self {
            id,
            parent_id: ctx.parent_id,
            address: ctx.address,
            is_window_focused: ctx.window.as_ref().map_or(false, |w| w.is_focused()),
            window: ctx.window,
            receives_all_mouse_events: false,
            parent_style: ctx.parent_style,
            self_style: None,
            is_focusable: false,
            is_focused: false,
            is_parent_enabled: ctx.is_parent_enabled,
            is_self_enabled: true,
            is_self_visible: true,
            is_mouse_over: false,
            enable_ime: false,
            rect_in_window: None,
            visible_rect: None,
            cursor_icon: CursorIcon::Default,
            children: BTreeMap::new(),
            layout_item_options: LayoutItemOptions::default(),
            size_hint_x_cache: HashMap::new(),
            size_hint_y_cache: HashMap::new(),
            size_x_fixed_cache: None,
            size_y_fixed_cache: None,
            is_accessible: true,
            pending_accessible_update: false,
            is_registered_as_focusable: false,
            event_filter: None,
            current_layout_event: None,
            is_window_root: ctx.is_window_root,
            grid_options: None,
            no_padding: false,
            shortcuts: Vec::new(),
            style_element,
            common_style,
        };

        if let Some(window) = &common.window {
            let root_widget_id = window.root_widget_id();
            window.accessible_mount(
                if common.id == root_widget_id {
                    None
                } else if let Some(parent_id) = common.parent_id {
                    Some(parent_id.into())
                } else {
                    warn!("widget is not a window root so it must have a parent");
                    None
                },
                common.id.into(),
                // TODO: calculate visual index instead
                common
                    .address
                    .path
                    .last()
                    .map(|(index, _id)| *index)
                    .unwrap_or_default() as usize,
            );
        }
        common.update();
        common.enabled_changed();
        common.focused_changed();
        common.mouse_over_changed();
        common.register_focusable();
        common.refresh_common_style();

        WidgetCommonTyped {
            common,
            _marker: PhantomData,
        }
    }

    pub fn window_id(&self) -> Option<WindowId> {
        self.window.as_ref().map(|w| w.id())
    }

    pub fn set_grid_options(&mut self, options: Option<GridOptions>) {
        self.grid_options = options;
        self.size_hint_changed();
    }

    pub fn set_no_padding(&mut self, value: bool) {
        self.no_padding = value;
        self.size_hint_changed();
    }

    pub fn grid_options(&self) -> GridOptions {
        self.grid_options.clone().unwrap_or_else(|| {
            let style = self.style();
            GridOptions {
                x: GridAxisOptions {
                    min_padding: if self.no_padding {
                        0
                    } else {
                        style.0.grid.min_padding.x
                    },
                    min_spacing: style.0.grid.min_spacing.x,
                    preferred_padding: if self.no_padding {
                        0
                    } else {
                        style.0.grid.preferred_padding.x
                    },
                    preferred_spacing: style.0.grid.preferred_spacing.x,
                    border_collapse: 0,
                    alignment: Alignment::Start,
                },
                y: GridAxisOptions {
                    min_padding: if self.no_padding {
                        0
                    } else {
                        style.0.grid.min_padding.y
                    },
                    min_spacing: style.0.grid.min_spacing.y,
                    preferred_padding: if self.no_padding {
                        0
                    } else {
                        style.0.grid.preferred_padding.y
                    },
                    preferred_spacing: style.0.grid.preferred_spacing.y,
                    border_collapse: 0,
                    alignment: Alignment::Start,
                },
            }
        })
    }

    pub fn is_self_visible(&self) -> bool {
        self.is_self_visible
    }

    pub fn is_self_enabled(&self) -> bool {
        self.is_self_enabled
    }

    pub fn is_enabled(&self) -> bool {
        self.is_parent_enabled && self.is_self_enabled
    }

    pub fn is_focusable(&self) -> bool {
        self.is_focusable && self.is_enabled()
    }

    pub fn is_focused(&self) -> bool {
        self.is_focused && self.is_window_focused && self.is_enabled()
    }

    pub fn style(&self) -> &ComputedStyle {
        self.self_style.as_ref().unwrap_or(&self.parent_style)
    }

    pub fn new_creation_context(
        &self,
        new_id: RawWidgetId,
        key: Key,
        root_of_window: Option<Window>,
    ) -> WidgetCreationContext {
        WidgetCreationContext {
            parent_id: Some(self.id),
            address: self.address.clone().join(key, new_id),
            is_window_root: root_of_window.is_some(),
            window: root_of_window.or_else(|| self.window.clone()),
            parent_style: self.style().clone(),
            is_parent_enabled: self.is_enabled(),
        }
    }

    pub fn size(&self) -> Option<Size> {
        self.rect_in_window.as_ref().map(|g| g.size)
    }

    // Request redraw and accessible update
    pub fn update(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        window.request_redraw();
        self.pending_accessible_update = true;
    }

    pub fn has_child(&self, key: Key) -> bool {
        self.children.contains_key(&key)
    }

    // TODO: check for row/column conflict
    // TODO: move options to child widget common
    pub fn child<T: Widget>(&mut self, key: Key) -> &mut T {
        let new_id = RawWidgetId::new();
        let ctx = if T::is_window_root_type() {
            let new_window = Window::new(new_id);
            self.new_creation_context(new_id, key, Some(new_window.clone()))
        } else {
            self.new_creation_context(new_id, key, None)
        };
        match self.children.entry(key) {
            btree_map::Entry::Vacant(entry) => entry.insert(Child {
                widget: Box::new(T::new(WidgetCommon::new::<T>(ctx))),
                rect_in_parent: None,
                rect_set_during_layout: false,
            }),
            btree_map::Entry::Occupied(entry) => {
                if entry.get().widget.is::<T>() {
                    entry.into_mut()
                } else {
                    let child = entry.into_mut();
                    // Deletes old widget.
                    *child = Child {
                        widget: Box::new(T::new(WidgetCommon::new::<T>(ctx))),
                        rect_in_parent: None,
                        rect_set_during_layout: false,
                    };
                    child
                }
            }
        };

        self.size_hint_changed();
        self.children
            .get_mut(&key)
            .unwrap()
            .widget
            .downcast_mut()
            .unwrap()
    }

    pub fn get_child<T: Widget>(&self, key: Key) -> anyhow::Result<&T> {
        self.children
            .get(&key)
            .context("no such key")?
            .widget
            .downcast_ref()
            .context("child type mismatch")
    }

    pub fn get_child_mut<T: Widget>(&mut self, key: Key) -> anyhow::Result<&mut T> {
        self.children
            .get_mut(&key)
            .context("no such key")?
            .widget
            .downcast_mut()
            .context("child type mismatch")
    }

    pub fn layout_item_options(&self) -> &LayoutItemOptions {
        &self.layout_item_options
    }

    pub fn set_layout_item_options(&mut self, options: LayoutItemOptions) {
        self.layout_item_options = options;
        self.size_hint_changed();
    }

    pub fn remove_child(&mut self, key: Key) -> Result<()> {
        self.children.remove(&key).context("no such key")?;
        self.size_hint_changed();
        Ok(())
    }

    pub fn set_child_rect(&mut self, key: Key, rect_in_parent: Option<Rect>) -> Result<()> {
        let child = self
            .children
            .get_mut(&key)
            .context("set_child_rect: invalid child index")?;
        if child.widget.common().is_window_root {
            bail!("cannot set child rect for child window");
        }

        let rect_in_window = if let Some(rect_in_window) = self.rect_in_window {
            rect_in_parent.map(|rect_in_parent| rect_in_parent.translate(rect_in_window.top_left))
        } else {
            None
        };
        let visible_rect = if let (Some(visible_rect), Some(rect_in_parent)) =
            (self.visible_rect, rect_in_parent)
        {
            Some(
                visible_rect
                    .translate(-rect_in_parent.top_left)
                    .intersect(Rect::from_pos_size(Point::default(), rect_in_parent.size)),
            )
            .filter(|r| r != &Rect::default())
        } else {
            None
        };
        child.rect_in_parent = rect_in_parent;
        // println!(
        //     "rect_in_window {:?} -> {:?}",
        //     child.widget.common().rect_in_window,
        //     rect_in_window
        // );
        // println!(
        //     "visible_rect {:?} -> {:?}",
        //     child.widget.common().visible_rect,
        //     visible_rect
        // );
        let rects_changed = child.widget.common().rect_in_window != rect_in_window
            || child.widget.common().visible_rect != visible_rect;
        if let Some(event) = &self.current_layout_event {
            if rects_changed || event.size_hints_changed_within(child.widget.common().address()) {
                //println!("set_child_rect ok1");
                child.widget.dispatch(
                    LayoutEvent {
                        new_rect_in_window: rect_in_window,
                        new_visible_rect: visible_rect,
                        changed_size_hints: event.changed_size_hints.clone(),
                    }
                    .into(),
                );
            }
            child.rect_set_during_layout = true;
        } else {
            if rects_changed {
                //println!("set_child_rect ok2");
                child.widget.dispatch(
                    LayoutEvent {
                        new_rect_in_window: rect_in_window,
                        new_visible_rect: visible_rect,
                        changed_size_hints: Vec::new(),
                    }
                    .into(),
                );
            }
        }
        //println!("set_child_rect end");
        Ok(())
    }

    pub fn set_child_rects(&mut self, rects: &BTreeMap<Key, Rect>) -> Result<()> {
        for (key, rect) in rects {
            self.set_child_rect(*key, Some(*rect))?;
        }
        Ok(())
    }

    pub fn size_hint_changed(&mut self) {
        self.clear_size_hint_cache();
        let Some(window) = &self.window else {
            return;
        };
        window.invalidate_size_hint(self.address.clone());
    }

    pub fn clear_size_hint_cache(&mut self) {
        self.size_hint_x_cache.clear();
        self.size_hint_y_cache.clear();
        self.size_x_fixed_cache = None;
        self.size_y_fixed_cache = None;
    }

    pub fn window_or_err(&self) -> Result<&Window> {
        self.window.as_ref().context("no window")
    }

    pub fn address(&self) -> &WidgetAddress {
        &self.address
    }

    pub fn rect_in_window_or_err(&self) -> Result<Rect> {
        self.rect_in_window
            .with_context(|| format!("no rect_in_window for {:?}", self.id))
    }

    pub fn size_or_err(&self) -> Result<Size> {
        Ok(self.rect_in_window.context("no rect_in_window")?.size)
    }

    pub fn rect_or_err(&self) -> Result<Rect> {
        Ok(Rect::from_pos_size(
            Point::default(),
            self.rect_in_window.context("no rect_in_window")?.size,
        ))
    }

    fn register_focusable(&mut self) {
        let is_focusable = self.is_focusable();
        if is_focusable != self.is_registered_as_focusable {
            if let Some(window) = &self.window {
                if is_focusable {
                    window.add_focusable_widget(self.address.clone(), self.id);
                } else {
                    window.remove_focusable_widget(self.address.clone(), self.id);
                }
                self.is_registered_as_focusable = is_focusable;
            } else {
                self.is_registered_as_focusable = false;
            }
        }
    }

    pub fn enabled_changed(&mut self) {
        self.register_focusable();
        self.focused_changed();
        self.mouse_over_changed();
        if self.is_enabled() {
            self.style_element
                .remove_pseudo_class(MyPseudoClass::Disabled);
            self.style_element.add_pseudo_class(MyPseudoClass::Enabled);
        } else {
            self.style_element
                .remove_pseudo_class(MyPseudoClass::Enabled);
            self.style_element.add_pseudo_class(MyPseudoClass::Disabled);
        }
        self.refresh_common_style();
    }

    pub fn focused_changed(&mut self) {
        if self.is_focused() {
            self.style_element.add_pseudo_class(MyPseudoClass::Focus);
        } else {
            self.style_element.remove_pseudo_class(MyPseudoClass::Focus);
        }
        self.refresh_common_style();
    }

    pub fn mouse_over_changed(&mut self) {
        if self.is_mouse_over {
            self.style_element.add_pseudo_class(MyPseudoClass::Hover);
        } else {
            self.style_element.remove_pseudo_class(MyPseudoClass::Hover);
        }
        self.refresh_common_style();
    }

    pub fn set_focusable(&mut self, focusable: bool) {
        self.is_focusable = focusable;
        self.register_focusable();
    }

    fn unmount_accessible(&mut self) {
        // println!("unmount_accessible {:?}", self.id);
        // for child in self.children.values_mut() {
        //     child.widget.common_mut().unmount_accessible();
        // }
        if let Some(window) = &self.window {
            let root_widget_id = window.root_widget_id();
            window.accessible_unmount(
                if self.id == root_widget_id {
                    None
                } else {
                    self.parent_id.map(|id| id.into())
                },
                self.id.into(),
            );
        }
    }

    pub fn widget<W: Widget>(&mut self, id: WidgetId<W>) -> Result<&mut W, WidgetNotFound> {
        let w = self.widget_raw(id.0)?;
        Ok(w.downcast_mut::<W>().expect("widget downcast failed"))
    }

    pub fn widget_raw(&mut self, id: RawWidgetId) -> Result<&mut dyn Widget, WidgetNotFound> {
        // TODO: speed up
        for child in self.children.values_mut() {
            if child.widget.common().id == id {
                return Ok(child.widget.as_mut());
            }
            if let Ok(widget) = child.widget.common_mut().widget_raw(id) {
                return Ok(widget);
            }
        }
        Err(WidgetNotFound)
    }

    pub fn add_shortcut(&mut self, shortcut: Shortcut) -> ShortcutId {
        let id = shortcut.id;
        if shortcut.scope == ShortcutScope::Application {
            with_system(|system| system.application_shortcuts.push(shortcut.clone()));
        }
        // TODO: register widget/window shortcuts
        self.shortcuts.push(shortcut);
        id
    }

    // TODO: remove_shortcut

    pub fn refresh_common_style(&mut self) {
        self.common_style = self.style().get_common(&self.style_element);
        self.size_hint_changed();
        self.update();
    }

    pub fn style_element(&self) -> &Element {
        &self.style_element
    }

    pub fn specific_style<T: ComputedElementStyle>(&self) -> Rc<T> {
        self.style().get(&self.style_element)
    }

    pub fn add_pseudo_class(&mut self, class: MyPseudoClass) {
        self.style_element.add_pseudo_class(class);
        self.refresh_common_style();
    }

    pub fn remove_pseudo_class(&mut self, class: MyPseudoClass) {
        self.style_element.remove_pseudo_class(class);
        self.refresh_common_style();
    }

    pub fn set_accessible(&mut self, value: bool) {
        if self.is_accessible == value {
            return;
        }
        self.is_accessible = value;
        self.update();
    }
}

#[derive(Debug)]
pub struct WidgetCommonTyped<T> {
    pub common: WidgetCommon,
    _marker: PhantomData<T>,
}

impl<W> WidgetCommonTyped<W> {
    pub fn id(&self) -> WidgetId<W> {
        WidgetId(self.common.id, PhantomData)
    }

    pub fn callback<E, F>(&self, func: F) -> Callback<E>
    where
        W: Widget,
        F: Fn(&mut W, E) -> Result<()> + 'static,
        E: 'static,
    {
        widget_callback(WidgetId::<W>::new(self.common.id), func)
    }

    pub fn add_child<T: Widget>(&mut self, key: Key) -> &mut T {
        self.common.child::<T>(key)
    }
}

impl<T> Deref for WidgetCommonTyped<T> {
    type Target = WidgetCommon;

    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl<T> DerefMut for WidgetCommonTyped<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.common
    }
}

impl<T> From<WidgetCommonTyped<T>> for WidgetCommon {
    fn from(value: WidgetCommonTyped<T>) -> Self {
        value.common
    }
}
