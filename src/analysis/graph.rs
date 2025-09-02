use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct DirectedGraph<NodeId> {
    nodes: HashSet<NodeId>,
    successors: HashMap<NodeId, Vec<NodeId>>,
    predecessors: HashMap<NodeId, Vec<NodeId>>,
}

impl<NodeId> DirectedGraph<NodeId>
where
    NodeId: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            successors: HashMap::new(),
            predecessors: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: NodeId) {
        self.nodes.insert(node.clone());
        self.successors.entry(node.clone()).or_default();
        self.predecessors.entry(node).or_default();
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId) {
        self.successors
            .entry(from.clone())
            .or_default()
            .push(to.clone());
        self.predecessors.entry(to).or_default().push(from);
    }

    pub fn successors(&self, node: &NodeId) -> &[NodeId] {
        self.successors.get(node).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn predecessors(&self, node: &NodeId) -> &[NodeId] {
        self.predecessors
            .get(node)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn nodes(&self) -> impl Iterator<Item = &NodeId> {
        self.nodes.iter()
    }
}

#[derive(Debug, Clone)]
pub struct Dominators<NodeId> {
    /// Maps each node to its immediate dominator (if any)
    immediate_dominators: HashMap<NodeId, NodeId>,
    /// Reverse postorder of nodes for efficient iteration
    reverse_postorder: Vec<NodeId>,
    /// The entry/root node of the graph
    entry: NodeId,
}

impl<NodeId> Dominators<NodeId>
where
    NodeId: Eq + Hash + Clone,
{
    /// Compute dominators using Cooper-Harvey-Kennedy algorithm
    pub fn compute(graph: &DirectedGraph<NodeId>, entry: NodeId) -> Self {
        // Step 1: Compute reverse postorder traversal starting from entry
        let reverse_postorder = Self::reverse_postorder(graph, &entry);

        // Step 2: Initialize immediate dominators
        let mut immediate_dominators = HashMap::new();

        // Entry dominates itself
        immediate_dominators.insert(entry.clone(), entry.clone());

        // Step 3: Iterative dataflow analysis using Cooper-Harvey-Kennedy algorithm
        let mut changed = true;
        while changed {
            changed = false;

            // Process nodes in reverse postorder (except entry)
            for node in &reverse_postorder {
                if *node == entry {
                    continue;
                }

                let predecessors = graph.predecessors(node);
                if predecessors.is_empty() {
                    continue; // Unreachable node
                }

                // Find first processed predecessor
                let mut new_idom = None;
                for pred in predecessors {
                    if immediate_dominators.contains_key(pred) {
                        new_idom = Some(pred.clone());
                        break;
                    }
                }

                if let Some(mut idom) = new_idom {
                    // Intersect with all other processed predecessors
                    for pred in predecessors {
                        if pred != &idom && immediate_dominators.contains_key(pred) {
                            idom = Self::intersect(
                                &immediate_dominators,
                                &reverse_postorder,
                                idom,
                                pred.clone(),
                            );
                        }
                    }

                    // Check if immediate dominator changed
                    if immediate_dominators.get(node) != Some(&idom) {
                        immediate_dominators.insert(node.clone(), idom);
                        changed = true;
                    }
                }
            }
        }

        Self {
            immediate_dominators,
            reverse_postorder,
            entry,
        }
    }

    /// Compute reverse postorder traversal from entry node
    fn reverse_postorder(graph: &DirectedGraph<NodeId>, entry: &NodeId) -> Vec<NodeId> {
        let mut visited = HashSet::new();
        let mut postorder = Vec::new();

        Self::postorder_dfs(graph, entry, &mut visited, &mut postorder);

        // Reverse to get reverse postorder
        postorder.reverse();
        postorder
    }

    /// Depth-first search to compute postorder
    fn postorder_dfs(
        graph: &DirectedGraph<NodeId>,
        node: &NodeId,
        visited: &mut HashSet<NodeId>,
        postorder: &mut Vec<NodeId>,
    ) {
        if visited.contains(node) {
            return;
        }
        visited.insert(node.clone());

        for successor in graph.successors(node) {
            Self::postorder_dfs(graph, successor, visited, postorder);
        }

        postorder.push(node.clone());
    }

    /// Intersect two dominators - find nearest common dominator
    fn intersect(
        immediate_dominators: &HashMap<NodeId, NodeId>,
        reverse_postorder: &[NodeId],
        mut finger1: NodeId,
        mut finger2: NodeId,
    ) -> NodeId {
        // Create position map for efficient lookup
        let mut positions = HashMap::new();
        for (i, node) in reverse_postorder.iter().enumerate() {
            positions.insert(node.clone(), i);
        }

        let pos1 = positions.get(&finger1).copied().unwrap_or(usize::MAX);
        let pos2 = positions.get(&finger2).copied().unwrap_or(usize::MAX);

        let mut pos_finger1 = pos1;
        let mut pos_finger2 = pos2;

        while finger1 != finger2 {
            while pos_finger1 > pos_finger2 {
                if let Some(idom) = immediate_dominators.get(&finger1) {
                    finger1 = idom.clone();
                    pos_finger1 = positions.get(&finger1).copied().unwrap_or(usize::MAX);
                } else {
                    break;
                }
            }

            while pos_finger2 > pos_finger1 {
                if let Some(idom) = immediate_dominators.get(&finger2) {
                    finger2 = idom.clone();
                    pos_finger2 = positions.get(&finger2).copied().unwrap_or(usize::MAX);
                } else {
                    break;
                }
            }
        }

        finger1
    }

    /// Returns true if `dominator` dominates `node`
    pub fn dominates(&self, dominator: &NodeId, node: &NodeId) -> bool {
        if dominator == node {
            return true;
        }

        let mut current = node.clone();
        while let Some(idom) = self.immediate_dominators.get(&current) {
            if idom == dominator {
                return true;
            }
            if idom == &current {
                break; // Reached entry node
            }
            current = idom.clone();
        }
        false
    }

    /// Returns true if `dominator` strictly dominates `node` (dominates but is not equal)
    pub fn strictly_dominates(&self, dominator: &NodeId, node: &NodeId) -> bool {
        *dominator != *node && self.dominates(dominator, node)
    }

    /// Returns the set of all dominators for a given node
    pub fn dominators_of(&self, node: &NodeId) -> HashSet<NodeId> {
        let mut dominators = HashSet::new();
        let mut current = node.clone();

        loop {
            dominators.insert(current.clone());
            if let Some(idom) = self.immediate_dominators.get(&current) {
                if idom == &current {
                    break; // Reached entry node
                }
                current = idom.clone();
            } else {
                break;
            }
        }

        dominators
    }

    /// Returns the immediate dominator of a node, if any
    pub fn immediate_dominator(&self, node: &NodeId) -> Option<&NodeId> {
        let idom = self.immediate_dominators.get(node)?;
        if idom == node {
            None // Entry node has no immediate dominator
        } else {
            Some(idom)
        }
    }

    /// Returns all nodes that are dominated by the given node
    pub fn dominated_by(&self, dominator: &NodeId) -> Vec<NodeId> {
        self.immediate_dominators
            .keys()
            .filter(|&node| self.dominates(dominator, node))
            .cloned()
            .collect()
    }

    /// Returns the dominator tree as a mapping from immediate dominator to children
    pub fn dominator_tree(&self) -> HashMap<NodeId, Vec<NodeId>> {
        let mut tree = HashMap::new();

        // Initialize with entry
        tree.insert(self.entry.clone(), Vec::new());

        for (node, idom) in &self.immediate_dominators {
            if idom != node {
                // Skip entry node
                tree.entry(idom.clone()).or_default().push(node.clone());
            }
            tree.entry(node.clone()).or_default(); // Ensure all nodes are in the tree
        }

        tree
    }

    /// Returns the entry node
    pub fn entry(&self) -> &NodeId {
        &self.entry
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_simple_dominator_analysis() {
//         let mut graph = DirectedGraph::new();

//         // Create a simple diamond-shaped CFG:
//         //   A
//         //  / \
//         // B   C
//         //  \ /
//         //   D

//         graph.add_node("A");
//         graph.add_node("B");
//         graph.add_node("C");
//         graph.add_node("D");

//         graph.add_edge("A", "B");
//         graph.add_edge("A", "C");
//         graph.add_edge("B", "D");
//         graph.add_edge("C", "D");

//         let dominators = Dominators::compute(&graph, "A");

//         // A dominates all nodes
//         assert!(dominators.dominates(&"A", &"A"));
//         assert!(dominators.dominates(&"A", &"B"));
//         assert!(dominators.dominates(&"A", &"C"));
//         assert!(dominators.dominates(&"A", &"D"));

//         // B only dominates itself
//         assert!(dominators.dominates(&"B", &"B"));
//         assert!(!dominators.dominates(&"B", &"A"));
//         assert!(!dominators.dominates(&"B", &"C"));
//         assert!(!dominators.dominates(&"B", &"D"));

//         // D is dominated by A and itself
//         let d_dominators = dominators.dominators_of(&"D");
//         assert_eq!(d_dominators.len(), 2);
//         assert!(d_dominators.contains(&"A"));
//         assert!(d_dominators.contains(&"D"));

//         // Immediate dominators
//         assert_eq!(dominators.immediate_dominator(&"B"), Some(&"A"));
//         assert_eq!(dominators.immediate_dominator(&"C"), Some(&"A"));
//         assert_eq!(dominators.immediate_dominator(&"D"), Some(&"A"));
//         assert_eq!(dominators.immediate_dominator(&"A"), None);
//     }

//     #[test]
//     fn test_linear_graph() {
//         let mut graph = DirectedGraph::new();

//         // A -> B -> C
//         graph.add_node("A");
//         graph.add_node("B");
//         graph.add_node("C");

//         graph.add_edge("A", "B");
//         graph.add_edge("B", "C");

//         let dominators = Dominators::compute(&graph, "A");

//         // Each node dominates all nodes after it
//         assert!(dominators.dominates(&"A", &"B"));
//         assert!(dominators.dominates(&"A", &"C"));
//         assert!(dominators.dominates(&"B", &"C"));

//         // Immediate dominators form a chain
//         assert_eq!(dominators.immediate_dominator(&"B"), Some(&"A"));
//         assert_eq!(dominators.immediate_dominator(&"C"), Some(&"B"));
//     }

//     #[test]
//     fn test_complex_graph() {
//         let mut graph = DirectedGraph::new();

//         // More complex CFG:
//         //     A
//         //    / \
//         //   B   C
//         //   |\ /|
//         //   | X |
//         //   |/ \|
//         //   D   E
//         //    \ /
//         //     F

//         graph.add_node("A");
//         graph.add_node("B");
//         graph.add_node("C");
//         graph.add_node("D");
//         graph.add_node("E");
//         graph.add_node("F");

//         graph.add_edge("A", "B");
//         graph.add_edge("A", "C");
//         graph.add_edge("B", "D");
//         graph.add_edge("B", "E");
//         graph.add_edge("C", "D");
//         graph.add_edge("C", "E");
//         graph.add_edge("D", "F");
//         graph.add_edge("E", "F");

//         let dominators = Dominators::compute(&graph, "A");

//         // A dominates everything
//         assert!(dominators.dominates(&"A", &"F"));

//         // F is dominated by A only (besides itself)
//         let f_dominators = dominators.dominators_of(&"F");
//         assert_eq!(f_dominators.len(), 2);
//         assert!(f_dominators.contains(&"A"));
//         assert!(f_dominators.contains(&"F"));

//         // F's immediate dominator should be A
//         assert_eq!(dominators.immediate_dominator(&"F"), Some(&"A"));
//     }
// }

// pub trait WithExitNodes {
//     type NodeId;
//     fn exit_nodes(&self) -> Vec<Self::NodeId>;
// }

// impl<NodeId> WithExitNodes for DirectedGraph<NodeId>
// where
//     NodeId: Eq + Hash + Clone,
// {
//     type NodeId = NodeId;

//     fn exit_nodes(&self) -> Vec<Self::NodeId> {
//         self.nodes()
//             .filter(|node| self.successors(node).is_empty())
//             .cloned()
//             .collect()
//     }
// }

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum ExtNode<NodeId> {
//     Real(Option<NodeId>),
//     Fake,
// }

// impl<NodeId> ExtNode<NodeId> {
//     pub fn is_none(&self) -> bool {
//         matches!(self, ExtNode::Real(None))
//     }
// }

// #[derive(Debug, Clone)]
// pub struct PostDominators<NodeId> {
//     /// Maps each node to its immediate post-dominator
//     immediate_post_dominators: HashMap<NodeId, ExtNode<NodeId>>,
//     /// Postorder positions for efficient intersection
//     postorder_positions: HashMap<NodeId, usize>,
//     /// Exit nodes of the graph
//     exit_nodes: Vec<NodeId>,
// }

// impl<NodeId> PostDominators<NodeId>
// where
//     NodeId: Eq + Hash + Clone,
// {
//     /// Compute post-dominators using adapted Cooper-Harvey-Kennedy algorithm
//     pub fn compute<G: WithExitNodes<NodeId = NodeId>>(graph: &DirectedGraph<NodeId>, _: &G) -> Self
//     where
//         G: WithExitNodes<NodeId = NodeId>,
//     {
//         let exit_nodes = graph.exit_nodes();

//         // Step 1: Compute reverse postorder traversal from exit nodes (going backwards)
//         let reverse_postorder = Self::postdom_reverse_postorder(graph, &exit_nodes);

//         // Create postorder positions map
//         let mut postorder_positions = HashMap::new();
//         for (i, node) in reverse_postorder.iter().rev().enumerate() {
//             postorder_positions.insert(node.clone(), i);
//         }

//         // Set exit nodes to have highest postorder rank
//         let exit_rank = reverse_postorder.len();
//         for exit_node in &exit_nodes {
//             postorder_positions.insert(exit_node.clone(), exit_rank);
//         }

//         // Step 2: Initialize immediate post-dominators
//         let mut immediate_post_dominators = HashMap::new();

//         // Initialize all nodes as unprocessed
//         for node in graph.nodes() {
//             immediate_post_dominators.insert(node.clone(), ExtNode::Real(None));
//         }

//         // Exit nodes post-dominate themselves
//         for exit_node in &exit_nodes {
//             immediate_post_dominators.insert(exit_node.clone(), ExtNode::Real(Some(exit_node.clone())));
//         }

//         // Step 3: Iterative dataflow analysis
//         let mut changed = true;
//         while changed {
//             changed = false;

//             // Process nodes in reverse postorder
//             for node in &reverse_postorder {
//                 if exit_nodes.contains(node) {
//                     continue;
//                 }

//                 let successors = graph.successors(node);
//                 if successors.is_empty() {
//                     continue; // This shouldn't happen if exit_nodes is correct
//                 }

//                 let mut new_ipdom = ExtNode::Real(None);

//                 for succ in successors {
//                     match immediate_post_dominators.get(succ).cloned().unwrap_or(ExtNode::Real(None)) {
//                         ExtNode::Real(Some(_)) => {
//                             new_ipdom = match new_ipdom {
//                                 ExtNode::Real(Some(current_ipdom)) => {
//                                     Self::intersect(
//                                         &postorder_positions,
//                                         &immediate_post_dominators,
//                                         &exit_nodes,
//                                         current_ipdom,
//                                         succ.clone(),
//                                     )
//                                 }
//                                 ExtNode::Real(None) => ExtNode::Real(Some(succ.clone())),
//                                 ExtNode::Fake => ExtNode::Fake,
//                             };
//                         }
//                         ExtNode::Real(None) => {
//                             // Successor not yet processed, skip
//                         }
//                         ExtNode::Fake => {
//                             new_ipdom = ExtNode::Fake;
//                         }
//                     }
//                 }

//                 if new_ipdom != immediate_post_dominators.get(node).cloned().unwrap_or(ExtNode::Real(None)) {
//                     immediate_post_dominators.insert(node.clone(), new_ipdom);
//                     changed = true;
//                 }
//             }
//         }

//         Self {
//             immediate_post_dominators,
//             postorder_positions,
//             exit_nodes,
//         }
//     }

//     /// Compute reverse postorder traversal from exit nodes (traversing predecessors)
//     fn postdom_reverse_postorder(graph: &DirectedGraph<NodeId>, exit_nodes: &[NodeId]) -> Vec<NodeId> {
//         let mut visited = HashSet::new();
//         let mut postorder = Vec::new();

//         // Start DFS from each exit node
//         for exit_node in exit_nodes {
//             Self::postdom_postorder_dfs(graph, exit_node, &mut visited, &mut postorder);
//         }

//         // Reverse to get reverse postorder
//         postorder.reverse();
//         postorder
//     }

//     /// DFS traversal following predecessors to compute postorder
//     fn postdom_postorder_dfs(
//         graph: &DirectedGraph<NodeId>,
//         node: &NodeId,
//         visited: &mut HashSet<NodeId>,
//         postorder: &mut Vec<NodeId>,
//     ) {
//         if visited.contains(node) {
//             return;
//         }
//         visited.insert(node.clone());

//         // Visit predecessors (going backwards in the graph)
//         for predecessor in graph.predecessors(node) {
//             Self::postdom_postorder_dfs(graph, predecessor, visited, postorder);
//         }

//         postorder.push(node.clone());
//     }

//     /// Intersect two post-dominators - find nearest common post-dominator
//     fn intersect(
//         postorder_positions: &HashMap<NodeId, usize>,
//         immediate_post_dominators: &HashMap<NodeId, ExtNode<NodeId>>,
//         exit_nodes: &[NodeId],
//         mut finger1: NodeId,
//         mut finger2: NodeId,
//     ) -> ExtNode<NodeId> {
//         while finger1 != finger2 {
//             if exit_nodes.contains(&finger1) && exit_nodes.contains(&finger2) {
//                 return ExtNode::Fake;
//             }

//             let pos1 = postorder_positions.get(&finger1).copied().unwrap_or(0);
//             let pos2 = postorder_positions.get(&finger2).copied().unwrap_or(0);

//             while pos1 < pos2 {
//                 match immediate_post_dominators.get(&finger1).cloned().unwrap_or(ExtNode::Real(None)) {
//                     ExtNode::Real(Some(n)) => finger1 = n,
//                     ExtNode::Real(None) | ExtNode::Fake => break,
//                 }
//             }

//             while pos2 < pos1 {
//                 match immediate_post_dominators.get(&finger2).cloned().unwrap_or(ExtNode::Real(None)) {
//                     ExtNode::Real(Some(n)) => finger2 = n,
//                     ExtNode::Real(None) | ExtNode::Fake => break,
//                 }
//             }
//         }

//         ExtNode::Real(Some(finger1))
//     }

//     /// Returns true if the node is reachable from any exit node
//     pub fn is_reachable(&self, node: &NodeId) -> bool {
//         match self.immediate_post_dominators.get(node).cloned().unwrap_or(ExtNode::Real(None)) {
//             ExtNode::Real(None) => false,
//             ExtNode::Real(Some(_)) => true,
//             ExtNode::Fake => true,
//         }
//     }

//     /// Returns the immediate post-dominator of a node
//     pub fn immediate_post_dominator(&self, node: &NodeId) -> ExtNode<NodeId> {
//         self.immediate_post_dominators.get(node).cloned().unwrap_or(ExtNode::Real(None))
//     }

//     /// Returns true if `dom` post-dominates `node`
//     pub fn is_post_dominated_by(&self, node: &NodeId, dom: &NodeId) -> bool {
//         if node == dom {
//             return true;
//         }

//         let mut current = node.clone();
//         loop {
//             match self.immediate_post_dominator(&current) {
//                 ExtNode::Real(Some(ipdom)) => {
//                     if ipdom == *dom {
//                         return true;
//                     }
//                     if ipdom == current {
//                         break; // Reached an exit node
//                     }
//                     current = ipdom;
//                 }
//                 ExtNode::Real(None) | ExtNode::Fake => break,
//             }
//         }
//         false
//     }

//     /// Returns all post-dominators of a node
//     pub fn post_dominators_of(&self, node: &NodeId) -> Vec<NodeId> {
//         let mut post_dominators = Vec::new();
//         let mut current = node.clone();

//         loop {
//             post_dominators.push(current.clone());
//             match self.immediate_post_dominator(&current) {
//                 ExtNode::Real(Some(ipdom)) => {
//                     if ipdom == current {
//                         break; // Reached an exit node
//                     }
//                     current = ipdom;
//                 }
//                 ExtNode::Real(None) | ExtNode::Fake => break,
//             }
//         }

//         post_dominators
//     }

//     /// Returns the exit nodes
//     pub fn exit_nodes(&self) -> &[NodeId] {
//         &self.exit_nodes
//     }
// }

pub trait WithExitNodes {
    type NodeId;
    fn exit_nodes(&self) -> Vec<Self::NodeId>;
}

impl<NodeId> WithExitNodes for DirectedGraph<NodeId>
where
    NodeId: Eq + Hash + Clone,
{
    type NodeId = NodeId;

    fn exit_nodes(&self) -> Vec<Self::NodeId> {
        self.nodes()
            .filter(|node| self.successors(node).is_empty())
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtNode<NodeId> {
    Real(Option<NodeId>),
    Fake,
}

impl<NodeId> ExtNode<NodeId> {
    pub fn is_none(&self) -> bool {
        matches!(self, ExtNode::Real(None))
    }
}

#[derive(Debug, Clone)]
pub struct PostDominators<NodeId> {
    /// Maps each node to its immediate post-dominator
    immediate_post_dominators: HashMap<NodeId, ExtNode<NodeId>>,
    /// Postorder positions for efficient intersection
    postorder_positions: HashMap<NodeId, usize>,
    /// Exit nodes of the graph
    exit_nodes: Vec<NodeId>,
}

impl<NodeId> PostDominators<NodeId>
where
    NodeId: Eq + Hash + Clone,
{
    /// Compute post-dominators using adapted Cooper-Harvey-Kennedy algorithm
    pub fn compute<G: WithExitNodes<NodeId = NodeId>>(graph: &DirectedGraph<NodeId>, _: &G) -> Self
    where
        G: WithExitNodes<NodeId = NodeId>,
    {
        let exit_nodes = graph.exit_nodes();

        // Step 1: Compute reverse postorder traversal from exit nodes (going backwards)
        let reverse_postorder = Self::postdom_reverse_postorder(graph, &exit_nodes);

        // Create postorder positions map
        let mut postorder_positions = HashMap::new();
        for (i, node) in reverse_postorder.iter().rev().enumerate() {
            postorder_positions.insert(node.clone(), i);
        }

        // Set exit nodes to have highest postorder rank
        let exit_rank = reverse_postorder.len();
        for exit_node in &exit_nodes {
            postorder_positions.insert(exit_node.clone(), exit_rank);
        }

        // Step 2: Initialize immediate post-dominators
        let mut immediate_post_dominators = HashMap::new();

        // Initialize all nodes as unprocessed
        for node in graph.nodes() {
            immediate_post_dominators.insert(node.clone(), ExtNode::Real(None));
        }

        // Exit nodes post-dominate themselves
        for exit_node in &exit_nodes {
            immediate_post_dominators
                .insert(exit_node.clone(), ExtNode::Real(Some(exit_node.clone())));
        }

        // Step 3: Iterative dataflow analysis
        let mut changed = true;
        while changed {
            changed = false;

            // Process nodes in reverse postorder
            for node in &reverse_postorder {
                if exit_nodes.contains(node) {
                    continue;
                }

                let successors = graph.successors(node);
                if successors.is_empty() {
                    continue; // This shouldn't happen if exit_nodes is correct
                }

                let mut new_ipdom = ExtNode::Real(None);

                for succ in successors {
                    match immediate_post_dominators
                        .get(succ)
                        .cloned()
                        .unwrap_or(ExtNode::Real(None))
                    {
                        ExtNode::Real(Some(_)) => {
                            new_ipdom = match new_ipdom {
                                ExtNode::Real(Some(current_ipdom)) => Self::intersect(
                                    &postorder_positions,
                                    &immediate_post_dominators,
                                    &exit_nodes,
                                    current_ipdom,
                                    succ.clone(),
                                ),
                                ExtNode::Real(None) => ExtNode::Real(Some(succ.clone())),
                                ExtNode::Fake => ExtNode::Fake,
                            };
                        }
                        ExtNode::Real(None) => {
                            // Successor not yet processed, skip
                        }
                        ExtNode::Fake => {
                            new_ipdom = ExtNode::Fake;
                        }
                    }
                }

                if new_ipdom
                    != immediate_post_dominators
                        .get(node)
                        .cloned()
                        .unwrap_or(ExtNode::Real(None))
                {
                    immediate_post_dominators.insert(node.clone(), new_ipdom);
                    changed = true;
                }
            }
        }

        Self {
            immediate_post_dominators,
            postorder_positions,
            exit_nodes,
        }
    }

    /// Compute reverse postorder traversal from exit nodes (traversing predecessors)
    fn postdom_reverse_postorder(
        graph: &DirectedGraph<NodeId>,
        exit_nodes: &[NodeId],
    ) -> Vec<NodeId> {
        let mut visited = HashSet::new();
        let mut postorder = Vec::new();

        // Start DFS from each exit node
        for exit_node in exit_nodes {
            Self::postdom_postorder_dfs(graph, exit_node, &mut visited, &mut postorder);
        }

        // Reverse to get reverse postorder
        postorder.reverse();
        postorder
    }

    /// DFS traversal following predecessors to compute postorder
    fn postdom_postorder_dfs(
        graph: &DirectedGraph<NodeId>,
        node: &NodeId,
        visited: &mut HashSet<NodeId>,
        postorder: &mut Vec<NodeId>,
    ) {
        if visited.contains(node) {
            return;
        }
        visited.insert(node.clone());

        // Visit predecessors (going backwards in the graph)
        for predecessor in graph.predecessors(node) {
            Self::postdom_postorder_dfs(graph, predecessor, visited, postorder);
        }

        postorder.push(node.clone());
    }

    /// Intersect two post-dominators - find nearest common post-dominator
    fn intersect(
        postorder_positions: &HashMap<NodeId, usize>,
        immediate_post_dominators: &HashMap<NodeId, ExtNode<NodeId>>,
        exit_nodes: &[NodeId],
        mut finger1: NodeId,
        mut finger2: NodeId,
    ) -> ExtNode<NodeId> {
        while finger1 != finger2 {
            if exit_nodes.contains(&finger1) && exit_nodes.contains(&finger2) {
                return ExtNode::Fake;
            }

            let pos1 = postorder_positions.get(&finger1).copied().unwrap_or(0);
            let pos2 = postorder_positions.get(&finger2).copied().unwrap_or(0);

            while pos1 < pos2 {
                match immediate_post_dominators
                    .get(&finger1)
                    .cloned()
                    .unwrap_or(ExtNode::Real(None))
                {
                    ExtNode::Real(Some(n)) => finger1 = n,
                    ExtNode::Real(None) | ExtNode::Fake => break,
                }
            }

            while pos2 < pos1 {
                match immediate_post_dominators
                    .get(&finger2)
                    .cloned()
                    .unwrap_or(ExtNode::Real(None))
                {
                    ExtNode::Real(Some(n)) => finger2 = n,
                    ExtNode::Real(None) | ExtNode::Fake => break,
                }
            }
        }

        ExtNode::Real(Some(finger1))
    }

    /// Returns true if the node is reachable from any exit node
    pub fn is_reachable(&self, node: &NodeId) -> bool {
        match self
            .immediate_post_dominators
            .get(node)
            .cloned()
            .unwrap_or(ExtNode::Real(None))
        {
            ExtNode::Real(None) => false,
            ExtNode::Real(Some(_)) => true,
            ExtNode::Fake => true,
        }
    }

    /// Returns the immediate post-dominator of a node
    pub fn immediate_post_dominator(&self, node: &NodeId) -> ExtNode<NodeId> {
        self.immediate_post_dominators
            .get(node)
            .cloned()
            .unwrap_or(ExtNode::Real(None))
    }

    /// Returns true if `dom` post-dominates `node`
    pub fn is_post_dominated_by(&self, node: &NodeId, dom: &NodeId) -> bool {
        if node == dom {
            return true;
        }

        let mut current = node.clone();
        loop {
            match self.immediate_post_dominator(&current) {
                ExtNode::Real(Some(ipdom)) => {
                    if ipdom == *dom {
                        return true;
                    }
                    if ipdom == current {
                        break; // Reached an exit node
                    }
                    current = ipdom;
                }
                ExtNode::Real(None) | ExtNode::Fake => break,
            }
        }
        false
    }

    /// Returns all post-dominators of a node
    pub fn post_dominators_of(&self, node: &NodeId) -> Vec<NodeId> {
        let mut post_dominators = Vec::new();
        let mut current = node.clone();

        loop {
            post_dominators.push(current.clone());
            match self.immediate_post_dominator(&current) {
                ExtNode::Real(Some(ipdom)) => {
                    if ipdom == current {
                        break; // Reached an exit node
                    }
                    current = ipdom;
                }
                ExtNode::Real(None) | ExtNode::Fake => break,
            }
        }

        post_dominators
    }

    /// Returns the exit nodes
    pub fn exit_nodes(&self) -> &[NodeId] {
        &self.exit_nodes
    }

    /// Find the nearest common post-dominator of two nodes
    pub fn nearest_common_post_dominator(&self, node1: &NodeId, node2: &NodeId) -> Option<NodeId> {
        if node1 == node2 {
            return Some(node1.clone());
        }

        let pd1 = self.post_dominators_of(node1);
        let pd2 = self.post_dominators_of(node2);

        // Find the first common post-dominator by comparing from the end (exit nodes)
        for n1 in pd1.iter().rev() {
            for n2 in pd2.iter().rev() {
                if n1 == n2 {
                    return Some(n1.clone());
                }
            }
        }
        None
    }

    /// Returns an iterator over all post-dominators of a node
    pub fn post_dominators_iter(&self, node: &NodeId) -> PostDominatorIter<NodeId> {
        PostDominatorIter {
            post_dominators: self,
            current: Some(node.clone()),
        }
    }
}

/// Iterator over post-dominators of a node
pub struct PostDominatorIter<'a, NodeId> {
    post_dominators: &'a PostDominators<NodeId>,
    current: Option<NodeId>,
}

impl<'a, NodeId> Iterator for PostDominatorIter<'a, NodeId>
where
    NodeId: Eq + Hash + Clone,
{
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current.take() {
            match self.post_dominators.immediate_post_dominator(&current) {
                ExtNode::Real(Some(ipdom)) => {
                    if ipdom != current {
                        self.current = Some(ipdom);
                    }
                }
                ExtNode::Real(None) | ExtNode::Fake => {
                    // End of chain
                }
            }
            Some(current)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests2 {
    use super::*;

    // #[test]
    // fn test_postdom_linear_graph() {
    //     let mut graph = DirectedGraph::new();

    //     // A -> B -> C
    //     graph.add_node("A");
    //     graph.add_node("B");
    //     graph.add_node("C");

    //     graph.add_edge("A", "B");
    //     graph.add_edge("B", "C");

    //     let postdominators = PostDominators::compute(&graph, &graph);

    //     // C post-dominates all nodes
    //     assert!(postdominators.is_post_dominated_by(&"A", &"C"));
    //     assert!(postdominators.is_post_dominated_by(&"B", &"C"));
    //     assert!(postdominators.is_post_dominated_by(&"C", &"C"));

    //     // B post-dominates A and itself
    //     assert!(postdominators.is_post_dominated_by(&"A", &"B"));
    //     assert!(postdominators.is_post_dominated_by(&"B", &"B"));
    //     assert!(!postdominators.is_post_dominated_by(&"C", &"B"));

    //     // Check immediate post-dominators
    //     assert_eq!(postdominators.immediate_post_dominator(&"A"), ExtNode::Real(Some("B")));
    //     assert_eq!(postdominators.immediate_post_dominator(&"B"), ExtNode::Real(Some("C")));
    //     assert_eq!(postdominators.immediate_post_dominator(&"C"), ExtNode::Real(Some("C")));

    //     // Check post-dominator chains
    //     let a_postdoms = postdominators.post_dominators_of(&"A");
    //     assert_eq!(a_postdoms, vec!["A", "B", "C"]);

    //     let b_postdoms = postdominators.post_dominators_of(&"B");
    //     assert_eq!(b_postdoms, vec!["B", "C"]);

    //     let c_postdoms = postdominators.post_dominators_of(&"C");
    //     assert_eq!(c_postdoms, vec!["C"]);
    // }

    // #[test]
    // fn test_postdom_diamond_graph() {
    //     let mut graph = DirectedGraph::new();

    //     // Create a diamond-shaped CFG:
    //     //   A
    //     //  / \
    //     // B   C
    //     //  \ /
    //     //   D

    //     graph.add_node("A");
    //     graph.add_node("B");
    //     graph.add_node("C");
    //     graph.add_node("D");

    //     graph.add_edge("A", "B");
    //     graph.add_edge("A", "C");
    //     graph.add_edge("B", "D");
    //     graph.add_edge("C", "D");

    //     let postdominators = PostDominators::compute(&graph, &graph);

    //     // D post-dominates all nodes
    //     assert!(postdominators.is_post_dominated_by(&"A", &"D"));
    //     assert!(postdominators.is_post_dominated_by(&"B", &"D"));
    //     assert!(postdominators.is_post_dominated_by(&"C", &"D"));
    //     assert!(postdominators.is_post_dominated_by(&"D", &"D"));

    //     // B and C don't post-dominate each other
    //     assert!(!postdominators.is_post_dominated_by(&"B", &"C"));
    //     assert!(!postdominators.is_post_dominated_by(&"C", &"B"));

    //     // A is not post-dominated by B or C
    //     assert!(!postdominators.is_post_dominated_by(&"A", &"B"));
    //     assert!(!postdominators.is_post_dominated_by(&"A", &"C"));

    //     // Check immediate post-dominators
    //     assert_eq!(postdominators.immediate_post_dominator(&"A"), ExtNode::Real(Some("D")));
    //     assert_eq!(postdominators.immediate_post_dominator(&"B"), ExtNode::Real(Some("D")));
    //     assert_eq!(postdominators.immediate_post_dominator(&"C"), ExtNode::Real(Some("D")));
    //     assert_eq!(postdominators.immediate_post_dominator(&"D"), ExtNode::Real(Some("D")));

    //     // Check post-dominator chains
    //     let a_postdoms = postdominators.post_dominators_of(&"A");
    //     assert_eq!(a_postdoms, vec!["A", "D"]);

    //     let b_postdoms = postdominators.post_dominators_of(&"B");
    //     assert_eq!(b_postdoms, vec!["B", "D"]);
    // }

    // #[test]
    // fn test_postdom_multiple_exits() {
    //     let mut graph = DirectedGraph::new();

    //     // Graph with multiple exit nodes:
    //     //   A
    //     //  / \
    //     // B   C  (both B and C are exits)

    //     graph.add_node("A");
    //     graph.add_node("B");
    //     graph.add_node("C");

    //     graph.add_edge("A", "B");
    //     graph.add_edge("A", "C");

    //     let postdominators = PostDominators::compute(&graph, &graph);

    //     // Neither B nor C post-dominate A (since there are multiple exit paths)
    //     assert!(!postdominators.is_post_dominated_by(&"A", &"B"));
    //     assert!(!postdominators.is_post_dominated_by(&"A", &"C"));

    //     // B and C post-dominate themselves
    //     assert!(postdominators.is_post_dominated_by(&"B", &"B"));
    //     assert!(postdominators.is_post_dominated_by(&"C", &"C"));

    //     // A should have a Fake immediate post-dominator (multiple exits)
    //     assert_eq!(postdominators.immediate_post_dominator(&"A"), ExtNode::Fake);
    //     assert_eq!(postdominators.immediate_post_dominator(&"B"), ExtNode::Real(Some("B")));
    //     assert_eq!(postdominators.immediate_post_dominator(&"C"), ExtNode::Real(Some("C")));

    //     // Check exit nodes
    //     let exits = postdominators.exit_nodes();
    //     assert_eq!(exits.len(), 2);
    //     assert!(exits.contains(&"B"));
    //     assert!(exits.contains(&"C"));
    // }

    #[test]
    fn test_postdom_complex_graph() {
        let mut graph = DirectedGraph::new();

        // More complex CFG:
        //     A
        //    / \
        //   B   C
        //   |\ /|
        //   | X |
        //   |/ \|
        //   D   E
        //    \ /
        //     F

        graph.add_node("A");
        graph.add_node("B");
        graph.add_node("C");
        graph.add_node("D");
        graph.add_node("E");
        graph.add_node("F");

        graph.add_edge("A", "B");
        graph.add_edge("A", "C");
        graph.add_edge("B", "D");
        graph.add_edge("B", "E");
        graph.add_edge("C", "D");
        graph.add_edge("C", "E");
        graph.add_edge("D", "F");
        graph.add_edge("E", "F");

        let postdominators = PostDominators::compute(&graph, &graph);

        // F post-dominates everything
        assert!(postdominators.is_post_dominated_by(&"A", &"F"));
        assert!(postdominators.is_post_dominated_by(&"B", &"F"));
        assert!(postdominators.is_post_dominated_by(&"C", &"F"));
        assert!(postdominators.is_post_dominated_by(&"D", &"F"));
        assert!(postdominators.is_post_dominated_by(&"E", &"F"));
        assert!(postdominators.is_post_dominated_by(&"F", &"F"));

        // Check that intermediate nodes don't post-dominate each other inappropriately
        assert!(!postdominators.is_post_dominated_by(&"B", &"C"));
        assert!(!postdominators.is_post_dominated_by(&"C", &"B"));
        assert!(!postdominators.is_post_dominated_by(&"D", &"E"));
        assert!(!postdominators.is_post_dominated_by(&"E", &"D"));

        // Check immediate post-dominators
        assert_eq!(
            postdominators.immediate_post_dominator(&"A"),
            ExtNode::Real(Some("F"))
        );
        assert_eq!(
            postdominators.immediate_post_dominator(&"B"),
            ExtNode::Real(Some("F"))
        );
        assert_eq!(
            postdominators.immediate_post_dominator(&"C"),
            ExtNode::Real(Some("F"))
        );
        assert_eq!(
            postdominators.immediate_post_dominator(&"D"),
            ExtNode::Real(Some("F"))
        );
        assert_eq!(
            postdominators.immediate_post_dominator(&"E"),
            ExtNode::Real(Some("F"))
        );
        assert_eq!(
            postdominators.immediate_post_dominator(&"F"),
            ExtNode::Real(Some("F"))
        );
    }

    #[test]
    fn test_postdom_unreachable_nodes() {
        let mut graph = DirectedGraph::new();

        // Graph with unreachable node:
        // A -> B -> C
        // D (isolated)

        graph.add_node("A");
        graph.add_node("B");
        graph.add_node("C");
        graph.add_node("D");

        graph.add_edge("A", "B");
        graph.add_edge("B", "C");

        let postdominators = PostDominators::compute(&graph, &graph);

        // D should be its own post-dominator (isolated exit)
        assert!(postdominators.is_post_dominated_by(&"D", &"D"));
        assert_eq!(
            postdominators.immediate_post_dominator(&"D"),
            ExtNode::Real(Some("D"))
        );

        // D doesn't post-dominate anything else
        assert!(!postdominators.is_post_dominated_by(&"A", &"D"));
        assert!(!postdominators.is_post_dominated_by(&"B", &"D"));
        assert!(!postdominators.is_post_dominated_by(&"C", &"D"));

        // Check that the main chain still works
        assert!(postdominators.is_post_dominated_by(&"A", &"C"));
        assert!(postdominators.is_post_dominated_by(&"B", &"C"));
    }

    #[test]
    fn test_postdom_iterator() {
        let mut graph = DirectedGraph::new();

        // A -> B -> C -> D
        graph.add_node("A");
        graph.add_node("B");
        graph.add_node("C");
        graph.add_node("D");

        graph.add_edge("A", "B");
        graph.add_edge("B", "C");
        graph.add_edge("C", "D");

        let postdominators = PostDominators::compute(&graph, &graph);

        // Test iterator for node A
        let a_postdoms: Vec<_> = postdominators.post_dominators_iter(&"A").collect();
        assert_eq!(a_postdoms, vec!["A", "B", "C", "D"]);

        // Test iterator for node C
        let c_postdoms: Vec<_> = postdominators.post_dominators_iter(&"C").collect();
        assert_eq!(c_postdoms, vec!["C", "D"]);

        // Test iterator for exit node D
        let d_postdoms: Vec<_> = postdominators.post_dominators_iter(&"D").collect();
        assert_eq!(d_postdoms, vec!["D"]);
    }

    #[test]
    fn test_nearest_common_post_dominator() {
        let mut graph = DirectedGraph::new();

        // Create a more complex CFG:
        //     A
        //   /   \
        //  B     C
        //  |    / \
        //  D   E   F
        //   \ | /
        //     G

        graph.add_node("A");
        graph.add_node("B");
        graph.add_node("C");
        graph.add_node("D");
        graph.add_node("E");
        graph.add_node("F");
        graph.add_node("G");

        graph.add_edge("A", "B");
        graph.add_edge("A", "C");
        graph.add_edge("B", "D");
        graph.add_edge("C", "E");
        graph.add_edge("C", "F");
        graph.add_edge("D", "G");
        graph.add_edge("E", "G");
        graph.add_edge("F", "G");

        let postdominators = PostDominators::compute(&graph, &graph);

        // Test nearest common post-dominator
        assert_eq!(
            postdominators.nearest_common_post_dominator(&"B", &"C"),
            Some("G")
        );
        assert_eq!(
            postdominators.nearest_common_post_dominator(&"D", &"E"),
            Some("G")
        );
        assert_eq!(
            postdominators.nearest_common_post_dominator(&"E", &"F"),
            Some("G")
        );
        assert_eq!(
            postdominators.nearest_common_post_dominator(&"A", &"G"),
            Some("G")
        );

        // Same node should return itself
        assert_eq!(
            postdominators.nearest_common_post_dominator(&"A", &"A"),
            Some("A")
        );
        assert_eq!(
            postdominators.nearest_common_post_dominator(&"G", &"G"),
            Some("G")
        );
    }
}
