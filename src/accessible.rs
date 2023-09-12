use std::collections::{HashMap, HashSet};

use accesskit::{Node, NodeClassSet, NodeId, Tree, TreeUpdate};

use crate::widgets::RawWidgetId;

pub struct AccessibleNodes {
    pub nodes: HashMap<NodeId, Node>,
    pub pending_updates: HashSet<NodeId>,
    pub classes: NodeClassSet,
    pub virtual_root: NodeId,
    pub root: NodeId,
    pub focus: NodeId,
}

impl AccessibleNodes {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let virtual_root = RawWidgetId::new().0.into();
        Self {
            nodes: Default::default(),
            pending_updates: Default::default(),
            classes: Default::default(),
            virtual_root,
            root: virtual_root,
            focus: virtual_root,
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.pending_updates.clear();
    }

    pub fn update(&mut self, id: NodeId, node: Option<Node>) {
        if let Some(node) = node {
            self.nodes.insert(id, node);
        } else {
            self.nodes.remove(&id);
        }
        self.pending_updates.insert(id);
    }

    pub fn take_update(&mut self) -> TreeUpdate {
        let mut nodes = Vec::new();
        for id in self.pending_updates.drain() {
            if let Some(node) = self.nodes.get(&id) {
                nodes.push((id, node.clone()));
            }
        }
        TreeUpdate {
            nodes,
            tree: Some(Tree { root: self.root }),
            focus: self.focus,
        }
    }
}
