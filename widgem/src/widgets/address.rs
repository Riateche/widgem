use {super::RawWidgetId, crate::child_key::ChildKey, std::fmt::Debug};

// TODO: store only keys?
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WidgetAddress {
    pub path: Vec<(ChildKey, RawWidgetId)>,
}

impl WidgetAddress {
    pub fn root(id: RawWidgetId) -> Self {
        Self {
            path: vec![("".into(), id)],
        }
    }
    pub fn join(mut self, key: ChildKey, id: RawWidgetId) -> Self {
        self.path.push((key, id));
        self
    }
    pub fn starts_with(&self, base: &WidgetAddress) -> bool {
        base.path.len() <= self.path.len() && base.path == self.path[..base.path.len()]
    }
    pub fn widget_id(&self) -> RawWidgetId {
        self.path.last().expect("WidgetAddress path is empty").1
    }
    pub fn parent_widget_id(&self) -> Option<RawWidgetId> {
        if self.path.len() > 1 {
            Some(self.path[self.path.len() - 2].1)
        } else {
            None
        }
    }
    pub fn strip_prefix(&self, parent: RawWidgetId) -> Option<&[(ChildKey, RawWidgetId)]> {
        if let Some(index) = self.path.iter().position(|(_index, id)| *id == parent) {
            Some(&self.path[index + 1..])
        } else {
            None
        }
    }
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.path.len()
    }
    pub fn item_at(&self, pos: usize) -> Option<(&ChildKey, &RawWidgetId)> {
        self.path.get(pos).map(|(x, y)| (x, y))
    }
}
