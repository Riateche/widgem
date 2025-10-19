use {crate::Context, derive_more::Deref, std::fmt::Display};

#[derive(Clone, Deref)]
pub struct Window {
    #[deref]
    inner: uitest::Window,
    context: Context,
}

impl Window {
    pub(crate) fn new(inner: uitest::Window, context: Context) -> Self {
        Self { inner, context }
    }

    pub fn snapshot(&self, text: impl Display) -> anyhow::Result<()> {
        self.context.check(|c| c.snapshot(self, text))
    }
}
