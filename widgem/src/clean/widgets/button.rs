use {
    crate::{
        clean::{
            render::WidgetRenderContext,
            widgets::{
                base::{WidgetBase, WidgetBaseExt},
                focusable::{FocusableAttrs, FocusableExt},
                text::Text,
                Grid,
            },
            Widget, WidgetExt,
        },
        layout::LayoutItemOptions,
        widgets::button::ComputedButtonStyle,
    },
    widgem_macros::AttributeSetters,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, AttributeSetters)]
pub struct Button {
    #[widgem_attr(extend = WidgetBaseExt)]
    base: WidgetBase,
    #[widgem_attr(extend = FocusableExt)]
    focusable: FocusableAttrs,
    #[widgem_attr(constructor)]
    text: String,
    #[widgem_attr(default = true)]
    auto_repeat: bool,
    #[widgem_attr(default = true)]
    mouse_leave_sensitive: bool,
    #[widgem_attr(default = true)]
    trigger_on_press: bool,

    #[widgem_attr(private)]
    pressed: bool,
    #[widgem_attr(private)]
    was_pressed_but_moved_out: bool,
}

impl Widget for Button {
    fn render(&self, ctx: WidgetRenderContext) -> crate::clean::BoxWidget {
        let style = ctx.compute_style::<ComputedButtonStyle>();
        Grid::new()
            .item(
                "text",
                LayoutItemOptions::default(),
                Text::new(self.text.clone()).boxed(),
            )
            .boxed()
    }
    //...
}

#[cfg(test)]
mod tests {
    use {
        crate::{
            clean::{
                render::WidgetRenderContext,
                widgets::{Button, FocusableExt, WidgetBaseExt},
                Widget,
            },
            style::{css::StyleSelector, defaults::default_style},
        },
        strict_num::FiniteF32,
    };

    #[test]
    fn test_button_attrs() {
        let button = Button::new("abc".into())
            .auto_repeat(false)
            .enabled(false)
            .focusable(true);
        let rendered = button.render(WidgetRenderContext {
            size: None,
            is_focused: false,
            is_under_mouse: false,
            is_effectively_enabled: true,
            is_effectively_visible: true,
            system_style: default_style(),
            custom_style: None,
            style_selector: StyleSelector::new("Button".into()),
            effective_scale: FiniteF32::new(1.0).unwrap(),
        });
        println!("OK {rendered:?}");
    }
}
