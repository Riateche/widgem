pub trait Widget: 'static {}

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

impl std::fmt::Debug for BoxWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: actual impl
        f.debug_tuple("BoxWidget").finish_non_exhaustive()
    }
}
