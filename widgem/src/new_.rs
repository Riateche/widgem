#![allow(dead_code)]

use {
    crate::{
        layout::LayoutItemOptions,
        shared_window::WindowId,
        types::{Axis, Point},
        ChildKey, RawWidgetId, WidgetGeometry,
    },
    std::collections::{BTreeMap, HashMap},
    winit::window::CursorIcon,
};

/*
Widget:
fn render(&self, ctx: RenderContext) -> BoxWidget {}

fn handle(&mut self, event: Event) -> Result<()> {}













*/

pub struct WidgetStyle;
pub struct KeyEvent;
pub struct MouseEvent;
pub struct MonitorInfo;
pub struct Clipboard;
pub struct TextInputAttrs;
pub struct TabOrder(pub u32);

pub trait WidgetAttrs {}
pub trait Widget {
    type State;
    fn render(&self, ctx: NonVisualWidgetContext) -> RenderedWidget;
    fn handle(&mut self, event: Event) -> anyhow::Result<bool>;
}

pub struct WidgetAttrsBox(pub Box<dyn WidgetAttrs>);

pub struct SystemContext {
    pub default_style: WidgetStyle,
    pub monitors: MonitorInfo,
    pub clipboard: Clipboard,
    pub windows: HashMap<WindowId, WindowContext>,
}

pub struct WindowContext {
    pub id: WindowId,
    pub winit_id: Option<winit::window::WindowId>,
    pub pos: Point,
}

// for visual widgets only;
// context is passed to widget when processing events and rendering
pub struct WidgetContext {
    pub system: SystemContext,
    pub window: WindowContext,
    pub geometry: Option<WidgetGeometry>,
    pub id: RawWidgetId,
    pub parent_id: Option<RawWidgetId>,
    pub is_focused: bool,
    pub is_focus_within: bool,
    pub is_window_focused: bool,
    pub is_under_mouse: bool,
    pub is_input_method_active: bool,
    pub is_effectively_enabled: bool, // self & parent enabled
    pub is_effectively_visible: bool, // self & parent visible
    pub is_window_root: bool,
    pub effective_style: WidgetStyle, // self or parent style
    pub effective_scale: f32,         // parent scale * (self scale OR 1)
}

pub struct NonVisualWidgetContext {
    pub system: SystemContext,
    pub id: RawWidgetId,
    pub parent_id: Option<RawWidgetId>,
}

pub struct NonVisualWidgetAttrs {
    id: RawWidgetId, // generated when state is created; public getter
}

// public attrs can be set by widget owner
pub struct CommonWidgetAttrs {
    id: RawWidgetId, // generated when state is created; public getter
    pub style: Option<WidgetStyle>,
    pub is_enabled: bool, // means self enabled
    pub is_visible: bool, // means self visible
    pub is_accessibility_node_enabled: bool,
    pub cursor_icon: Option<CursorIcon>, // effective icon is attrs.cursor_icon OR default widget icon OR parent icon
    pub scale: Option<f32>,
}

// common attrs for all focusable widgets
pub struct FocusableAttrs {
    pub is_focusable: bool,
    pub tab_order: TabOrder,
}

// attrs contain _all_ state of the widget;
// public attrs can be set by widget owner
pub struct ButtonAttrs {
    pub common: CommonWidgetAttrs,
    pub focusable: FocusableAttrs,
    pub text: String,
    pub auto_repeat: bool,
    pub is_mouse_leave_sensitive: bool,
    pub trigger_on_press: bool,

    pub is_pressed: bool,
    // widgets can have hidden internal state
    was_pressed_but_moved_out: bool,
}

pub enum ButtonEvent {
    Triggered(ButtonTriggeredEvent),
}

pub struct ButtonTriggeredEvent {
    pub kind: ButtonTriggeredEventKind,
}

pub enum ButtonTriggeredEventKind {
    Accessibility,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Programmatic,
}

pub struct WindowAttrs {
    // pos is not attr - it isn't fully controlled by owner
    pub title: String,
    pub decorations: bool,
    // ... and other winit attrs
    pub content: Option<WidgetAttrsBox>,
}

// NOTE: layout doesn't have state, so it can be a helper function
// instead of widget

// pub struct BoxLayoutAttrs<Key, W> {
//     pub axis: Axis,
//     pub items: BTreeMap<Key, RowLayoutItem<W>>,
// }

// pub struct RowLayoutItem<W> {
//     pub options: LayoutItemOptions,
//     pub widget: W,
// }

// example root
pub struct MyRoot {
    //id: WidgetId<Self>,
    //common: NonVisualWidgetAttrs,
    // initial root state is created when launching the app,
    // possibly with inputs
    pub config: String,
    // any "business logic" state needs to be here;
    //tasks: Vec<String>,
    // --also it contains state for any widget that's rendered by this widget--
    // window: WindowAttrs,
    // increment_button: ButtonAttrs,
    // name_text_input: TextInputAttrs,
    increment_ref: WidgetRef<Button>,
}

/*
Widget rendering:
inputs: widget context (read only), widget state (read only)
output: widget content expressed as widget attributes of child widgets.

Widget rendering is repeated until only the elementary widgets remain:
* timers (with callbacks)
* windows
* layouts and layout items
* text, images, lines, boxes, etc.
* colors
* clip bounds.

When an accessible widget is encountered,
its node is stored before the widget is replaced with the result of its render.

Layout stage calculates scale, size, position, clip bounds of all elementary widgets.

When handling mouse events, elementary widgets are mapped back to their owners.

--

Widget event handler: processes event and mutates state.







*/

impl Widget for MyRoot {
    //type State = MyRootState;
    fn render(&self, ctx: RenderContext) -> RenderedWidget {
        // self contains attribute values.
        // ctx contains system info, window info, parent info, widget states, geometries, etc.

        // state is unique to each widget instance.
        let state = ctx.state(ctx.id::<Self>());

        // we can also access any child widget's state.
        let increment_state = ctx.state(self.increment_ref);

        Window::new().title(format!("hello {}", self.name)).content(
            Row::new()
                .padding(1.lpx())
                .item(
                    "increment",
                    item_attrs,
                    Button::new()
                        .ref_(self.increment_ref)
                        .enabled(self.value > 0)
                        // macro generates static var with id for equivalence checks
                        .on_triggered(callback!(|this| this.increment())),
                )
                .item("text", item_attrs2, TextInput::new()),
        );
    }

    fn handle_mouse_input(&mut self, ctx: EventContext) -> anyhow::Result<bool> {
        // ctx contains system info, window info, parent info, widget states, geometries, etc.
        // EventContext also contains event data.

        // state is unique to each widget instance.
        let state = ctx.state(ctx.id::<Self>());

        // we can also access any child widget's state.
        let increment_state = ctx.state(self.increment_ref);

        Ok(false)
    }
}
