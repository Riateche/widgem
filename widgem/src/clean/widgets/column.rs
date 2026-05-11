use {
    crate::{
        clean::{
            widgets::{WidgetBase, WidgetBaseExt},
            BoxWidget,
        },
        layout::LayoutItemOptions,
        ChildKey,
    },
    widgem_macros::AttributeSetters,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LayoutItem {
    key: ChildKey,
    options: LayoutItemOptions,
    widget: BoxWidget,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, AttributeSetters)]
pub struct Column {
    #[widgem_attr(extend = WidgetBaseExt)]
    base: WidgetBase,
    #[widgem_attr(private)]
    items: Vec<LayoutItem>,
}

impl Column {
    pub fn item(mut self, key: ChildKey, options: LayoutItemOptions, widget: BoxWidget) -> Self {
        self.items.push(LayoutItem {
            key,
            options,
            widget,
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        clean::{
            widgets::{column::Column, Button, FocusableExt, WidgetBaseExt},
            WidgetExt,
        },
        layout::LayoutItemOptions,
    };

    #[test]
    fn test_column_attrs() {
        Column::new().item(
            "abc".into(),
            LayoutItemOptions::default(),
            Button::new("abc".into())
                .auto_repeat(false)
                .enabled(false)
                .focusable(true)
                .boxed(),
        );
    }
}
