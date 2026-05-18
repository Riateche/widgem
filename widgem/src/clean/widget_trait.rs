use {
    crate::clean::render::WidgetRenderContext,
    std::{
        any::Any,
        fmt::Debug,
        ops::{Deref, DerefMut},
    },
};

pub trait WidgetAuto: 'static + Debug + Any {
    fn type_name(&self) -> &'static str;
}

pub trait Widget: WidgetAuto {
    fn render(&self, ctx: WidgetRenderContext) -> BoxWidget;
}

#[derive(Debug)]
pub struct BoxWidget(pub(crate) Box<dyn Widget>);

impl std::hash::Hash for BoxWidget {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {
        todo!()
    }
}

impl Eq for BoxWidget {}

impl PartialEq for BoxWidget {
    fn eq(&self, _other: &Self) -> bool {
        todo!()
    }
}

impl Clone for BoxWidget {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl Deref for BoxWidget {
    type Target = dyn Widget;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for BoxWidget {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

// impl Debug for BoxWidget {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         // TODO: actual impl
//         f.debug_tuple("BoxWidget").finish_non_exhaustive()
//     }
// }
