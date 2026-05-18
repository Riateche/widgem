use {
    crate::{
        clean::{
            render::WidgetRenderContext,
            widgets::{Collection, WidgetBase, WidgetBaseExt},
            BoxWidget, Widget, WidgetExt,
        },
        layout::Alignment,
        ChildKey,
    },
    std::ops::RangeInclusive,
    widgem_macros::AttributeSetters,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GridItemOptions {
    x: GridItemAxisOptions,
    y: GridItemAxisOptions,
}

impl GridItemOptions {
    pub fn new(grid_cell_x: i32, grid_cell_y: i32) -> Self {
        Self::new_range(grid_cell_x..=grid_cell_x, grid_cell_y..=grid_cell_y)
    }

    pub fn new_range(grid_cell_x: RangeInclusive<i32>, grid_cell_y: RangeInclusive<i32>) -> Self {
        Self {
            x: GridItemAxisOptions {
                grid_cell: grid_cell_x,
                alignment: None,
                is_fixed: None,
            },
            y: GridItemAxisOptions {
                grid_cell: grid_cell_y,
                alignment: None,
                is_fixed: None,
            },
        }
    }

    pub fn alignment_x(mut self, alignment: Option<Alignment>) -> Self {
        self.x.alignment = alignment;
        self
    }

    pub fn alignment_y(mut self, alignment: Option<Alignment>) -> Self {
        self.y.alignment = alignment;
        self
    }

    pub fn fixed_x(mut self, fixed: Option<bool>) -> Self {
        self.x.is_fixed = fixed;
        self
    }

    pub fn fixed_y(mut self, fixed: Option<bool>) -> Self {
        self.y.is_fixed = fixed;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GridItemAxisOptions {
    // row or column
    grid_cell: RangeInclusive<i32>,
    alignment: Option<Alignment>,
    is_fixed: Option<bool>,
    // TODO: alignment, priority, stretch, etc.
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GridItem {
    key: ChildKey,
    options: GridItemOptions,
    widget: BoxWidget,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, AttributeSetters)]
pub struct Grid {
    #[widgem_attr(extend = WidgetBaseExt)]
    base: WidgetBase,
    #[widgem_attr(private)]
    items: Vec<GridItem>,
}

impl Grid {
    pub fn item(
        mut self,
        key: impl Into<ChildKey>,
        options: GridItemOptions,
        widget: BoxWidget,
    ) -> Self {
        self.items.push(GridItem {
            key: key.into(),
            options,
            widget,
        });
        self
    }
}

impl Widget for Grid {
    fn render(&self, _ctx: WidgetRenderContext) -> BoxWidget {
        Collection::new()
            .items(
                self.items
                    .iter()
                    .map(|item| (item.key.clone(), item.widget.clone())),
            )
            .boxed()
    }
}
