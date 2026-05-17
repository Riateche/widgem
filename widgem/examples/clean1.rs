use widgem::clean::{
    widgets::{Button, Window},
    BoxWidget, Widget, WidgetExt, WidgetRenderContext,
};

#[derive(Debug)]
struct Root;

impl Widget for Root {
    fn render(&self, _ctx: WidgetRenderContext) -> BoxWidget {
        Window::new("hello button".into())
            .content(Some(Button::new("hello".into()).boxed()))
            .boxed()
    }
}

fn main() -> anyhow::Result<()> {
    widgem::clean::run(Root.boxed())
}
