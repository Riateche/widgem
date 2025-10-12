use {
    crate::{child_key::ChildKey, RawWidgetId},
    accesskit::{Node, NodeId, Role, Tree, TreeUpdate},
    derivative::Derivative,
    std::collections::{hash_map::Entry, HashMap, HashSet},
    tracing::{error, warn},
};

#[derive(Debug)]
enum NodeKind {
    Real(Node),
    Hidden,
}

impl NodeKind {
    pub fn is_real(&self) -> bool {
        matches!(self, Self::Real(_))
    }

    #[allow(dead_code)]
    pub fn is_hidden(&self) -> bool {
        matches!(self, Self::Hidden)
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct AccessibilityNodes {
    nodes: HashMap<NodeId, NodeKind>,
    // TODO: BTreeMap? sort by visible row+column?
    // parent node id -> [(child key, node id)]
    direct_children: HashMap<NodeId, Vec<(ChildKey, NodeId)>>,
    // child node id -> parent node id
    direct_parents: HashMap<NodeId, NodeId>,

    pending_updates: HashSet<NodeId>,
    root: NodeId,
    focus: NodeId,
}

impl AccessibilityNodes {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let root_id = new_accessibility_node_id();
        let root_node = Node::new(Role::Group);
        Self {
            nodes: [(root_id, NodeKind::Real(root_node))].into(),
            direct_children: Default::default(),
            direct_parents: Default::default(),
            pending_updates: [root_id].into(),
            root: root_id,
            focus: root_id,
        }
    }

    pub fn safe_focus(&self) -> NodeId {
        let Some(data) = self.nodes.get(&self.focus) else {
            error!("AccessibilityNodes::safe_focus: unknown node is focused");
            return self.root;
        };
        if data.is_real() {
            self.focus
        } else {
            self.root
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn add_node(&mut self, parent: Option<NodeId>, child: NodeId, key_in_parent: ChildKey) {
        if let Some(parent) = parent {
            if !self.nodes.contains_key(&parent) {
                error!("AccessibilityNodes::add_node: parent does not exist");
                return;
            }
        }
        if self.nodes.contains_key(&child) {
            error!("AccessibilityNodes::add_node: child already exists");
            return;
        }
        self.nodes.insert(child, NodeKind::Hidden);
        let parent = parent.unwrap_or(self.root);
        let old_entry = self.direct_parents.insert(child, parent);
        if old_entry.is_some() {
            error!("AccessibilityNodes::add_node: direct_parents had conflicting entry");
        }
        let children = self.direct_children.entry(parent).or_default();
        let index = match children.binary_search_by_key(&&key_in_parent, |i| &i.0) {
            Ok(_) => {
                error!(
                    "AccessibilityNodes::add_node: direct_children already has a conflicting entry"
                );
                return;
            }
            Err(index) => index,
        };
        children.insert(index, (key_in_parent, child));
        self.mark_parent_as_pending(parent);
    }

    pub fn remove_node(&mut self, parent: Option<NodeId>, child: NodeId) {
        if let Some(parent) = parent {
            if !self.nodes.contains_key(&parent) {
                error!("AccessibilityNodes::remove_node: parent does not exist");
                return;
            }
        }
        if self.nodes.remove(&child).is_none() {
            error!("AccessibilityNodes::remove_node: child doesn't exist");
            return;
        }
        let parent = parent.unwrap_or(self.root);
        if self.direct_parents.remove(&child).is_none() {
            error!("AccessibilityNodes::remove_node: missing direct_parents entry");
        }
        if let Entry::Occupied(mut entry) = self.direct_children.entry(parent) {
            if let Some(index) = entry.get_mut().iter().position(|(_, id)| *id == child) {
                entry.get_mut().remove(index);
                if entry.get_mut().is_empty() {
                    entry.remove();
                }
            } else {
                error!("AccessibilityNodes::remove_node: missing direct_children entry (1)");
            }
        } else {
            error!("AccessibilityNodes::remove_node: missing direct_children entry (2)");
        }
        if self.focus == child {
            self.focus = self.root;
        }
        self.mark_parent_as_pending(parent);
    }

    fn mark_parent_as_pending(&mut self, mut parent: NodeId) {
        loop {
            if let Some(node) = self.nodes.get(&parent) {
                match node {
                    NodeKind::Real(_) => {
                        self.pending_updates.insert(parent);
                        return;
                    }
                    NodeKind::Hidden => {
                        if parent == self.root {
                            warn!("mark_parent_as_pending: root cannot be hidden");
                            self.pending_updates.insert(parent);
                            return;
                        }
                        if let Some(next) = self.direct_parents.get(&parent) {
                            parent = *next;
                        } else {
                            error!("parent not found for {:?}", parent);
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn update_node(&mut self, id: NodeId, data: Option<Node>) {
        let Some(node_data) = self.nodes.get_mut(&id) else {
            error!("AccessibilityNodes::update: unknown id");
            return;
        };
        let was_real = node_data.is_real();
        *node_data = if let Some(node) = data {
            NodeKind::Real(node)
        } else {
            NodeKind::Hidden
        };
        let is_real = node_data.is_real();

        self.pending_updates.insert(id);
        if was_real != is_real && id != self.root {
            if let Some(parent) = self.direct_parents.get(&id) {
                self.mark_parent_as_pending(*parent);
            } else {
                error!("parent not found for {:?}", id);
            }
        }
    }

    pub fn set_focus(&mut self, focus: Option<NodeId>) {
        if let Some(focus) = focus {
            if !self.nodes.contains_key(&focus) {
                error!("AccessibilityNodes::set_focus: id does not exist");
                self.focus = self.root;
                return;
            }
        }
        self.focus = focus.unwrap_or(self.root);
    }

    pub fn initial_empty_update(&self) -> TreeUpdate {
        let root_node = self
            .nodes
            .get(&self.root)
            .and_then(|data| match data {
                NodeKind::Real(node) => Some(node.clone()),
                NodeKind::Hidden => None,
            })
            .unwrap_or_else(|| {
                error!("AccessibilityNodes: missing root node");
                Node::new(Role::Group)
            });
        TreeUpdate {
            nodes: vec![(self.root, root_node)],
            // TODO: set Tree properties?
            tree: Some(Tree::new(self.root)),
            focus: self.root,
        }
    }

    pub fn subsequent_empty_update(&self) -> TreeUpdate {
        TreeUpdate {
            nodes: Vec::new(),
            // TODO: set Tree properties?
            tree: Some(Tree::new(self.root)),
            // TODO: this can fail if adapter doesn't have the focused node yet
            focus: self.safe_focus(),
        }
    }

    pub fn take_update(&mut self) -> TreeUpdate {
        let mut nodes = Vec::new();
        for id in self.pending_updates.drain() {
            if let Some(NodeKind::Real(node)) = self.nodes.get(&id) {
                let mut children = Vec::new();
                find_children(id, &self.direct_children, &self.nodes, &mut children);
                let mut node = node.clone();
                node.set_children(children);
                nodes.push((id, node));
            }
        }
        TreeUpdate {
            nodes,
            // TODO: set Tree properties?
            tree: Some(Tree::new(self.root)),
            focus: self.safe_focus(),
        }
    }
}

fn find_children(
    parent: NodeId,
    direct_children: &HashMap<NodeId, Vec<(ChildKey, NodeId)>>,
    nodes: &HashMap<NodeId, NodeKind>,
    out: &mut Vec<NodeId>,
) {
    if let Some(children) = direct_children.get(&parent) {
        for (_, child_id) in children {
            let Some(child_data) = nodes.get(child_id) else {
                error!("find_children: missing entry in nodes");
                continue;
            };
            if child_data.is_real() {
                out.push(*child_id);
            } else {
                find_children(*child_id, direct_children, nodes, out);
            }
        }
    }
}

pub fn new_accessibility_node_id() -> NodeId {
    RawWidgetId::new_unique().into()
}
