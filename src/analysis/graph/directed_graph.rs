use std::{collections::{HashMap, HashSet}, hash::Hash};

// Represents a directed graph using an adjacency list for successors
pub trait DirectedGraph {
    type Item;

    fn nodes(&self) -> impl Iterator<Item = Self::Item>;
    fn successors(&self) -> &HashMap<Self::Item, Vec<Self::Item>>;
    fn start_node(&self) -> Self::Item;
}


/// Computes the predecessors for every node in the graph.
///
/// Returns a map where each key is a node and the value is a vector
/// of its predecessor nodes.
pub fn compute_predecessors<NodeIdx: Eq + Hash + Copy, G: DirectedGraph<Item = NodeIdx>>(
    graph: &G
) -> HashMap<NodeIdx, Vec<NodeIdx>> {
    let mut predecessors: HashMap<NodeIdx, Vec<NodeIdx>> = HashMap::new();

    // Ensure every node has at least an empty Vec in the map
    for node in graph.nodes() {
        predecessors.entry(node).or_default();
    }

    // Iterate over each node and its successors to build the reverse mapping
    for (&node, successors) in graph.successors() {
        for &successor in successors {
            predecessors.entry(successor).or_default().push(node);
        }
    }

    predecessors
}


/// Computes the set of dominators for every node using an iterative algorithm.
///
/// Returns a map where each key is a node and the value is a HashSet
/// of the nodes that dominate it.
fn compute_dominators<NodeIdx: Eq + Hash + Copy, G: DirectedGraph<Item = NodeIdx>>(
    graph: &G, predecessors: &HashMap<NodeIdx, Vec<NodeIdx>>,
) -> HashMap<NodeIdx, HashSet<NodeIdx>> {
    // The dominator algorithm requires the predecessors of each node.
    let all_nodes: HashSet<NodeIdx> = graph.nodes().collect();

    // 1. Initialize dominator sets.
    let mut dominators: HashMap<NodeIdx, HashSet<NodeIdx>> = HashMap::new();
    let start_node = graph.start_node();

    for node in &all_nodes {
        if *node == start_node {
            // The start node is only dominated by itself.
            dominators.insert(*node, HashSet::from([start_node]));
        } else {
            // Initially, assume all nodes dominate every other node.
            dominators.insert(*node, all_nodes.clone());
        }
    }

    // 2. Iterate until the solution converges (no changes are made in a full pass).
    loop {
        let mut changed = false;

        for &node in &all_nodes {
            if node == start_node {
                continue; // The dominator of the start node is fixed.
            }

            let node_predecessors = match predecessors.get(&node) {
                Some(preds) => preds,
                // A node with no predecessors (that isn't the start node)
                // is unreachable. Its dominator set remains as all nodes.
                None => continue,
            };

            // Calculate the intersection of the dominators of all predecessors.
            // Start with a copy of the dominator set of the first predecessor.
            let mut new_dom_set = node_predecessors.get(0)
                .map(|p| dominators.get(p).unwrap().clone())
                .unwrap_or_else(HashSet::new);

            // Then, intersect it with the dominator sets of the other predecessors.
            for pred in node_predecessors.iter().skip(1) {
                if let Some(pred_doms) = dominators.get(pred) {
                    new_dom_set.retain(|d| pred_doms.contains(d));
                }
            }

            // Apply the formula: D(n) = {n} U intersection(...)
            new_dom_set.insert(node);

            // 3. Check if the dominator set has changed.
            if let Some(current_dom_set) = dominators.get_mut(&node) {
                if *current_dom_set != new_dom_set {
                    *current_dom_set = new_dom_set;
                    changed = true;
                }
            }
        }

        if !changed {
            break; // The solution has stabilized.
        }
    }

    dominators
}

#[cfg(test)]
mod tests {

    use super::*;

    type NodeId = usize;

    struct TestGraph {
        nodes: Vec<NodeId>,
        successors: HashMap<NodeId, Vec<NodeId>>,
        start_node: NodeId
    }

    // Helper methods for building a TestGraph instance
    impl TestGraph {
        fn new(start_node: NodeId) -> Self {
            Self {
                nodes: vec![start_node],
                successors: HashMap::new(),
                start_node,
            }
        }

        fn add_node(&mut self, node: NodeId) {
            if !self.nodes.contains(&node) {
                self.nodes.push(node);
            }
        }

        fn add_edge(&mut self, from: NodeId, to: NodeId) {
            self.add_node(from);
            self.add_node(to);
            self.successors.entry(from).or_default().push(to);
        }
    }

    impl DirectedGraph for TestGraph {
        type Item = NodeId;

        fn nodes(&self) -> impl Iterator<Item = NodeId> {
            self.nodes.iter().cloned()
        }

        fn successors(&self) -> &HashMap<NodeId, Vec<NodeId>> {
            &self.successors
        }

        fn start_node(&self) -> NodeId {
            self.start_node
        }
    }

    #[test]
    fn test_predecessors_one() {
        let graph = TestGraph::new(0);
        let mut result = compute_predecessors(&graph);

        // Sort for consistent comparison
        result.values_mut().for_each(|v| v.sort());

        let expected: HashMap<NodeId, Vec<NodeId>> = HashMap::from([
            (0, vec![]),
        ]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_predecessors_simple() {
        let mut graph = TestGraph::new(0);
        graph.add_edge(0, 1);
        graph.add_edge(1, 2);

        let mut result = compute_predecessors(&graph);
        result.values_mut().for_each(|v| v.sort());

        let expected: HashMap<NodeId, Vec<NodeId>> = HashMap::from([
            (0, vec![]),
            (1, vec![0]),
            (2, vec![1]),
        ]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_predecessors_loop() {
        let mut graph = TestGraph::new(0);
        graph.add_edge(0, 1);
        graph.add_edge(1, 0);

        let mut result = compute_predecessors(&graph);
        result.values_mut().for_each(|v| v.sort());

        let expected: HashMap<NodeId, Vec<NodeId>> = HashMap::from([
            (0, vec![1]),
            (1, vec![0]),
        ]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_predecessors_diamond() {
        let mut graph = TestGraph::new(0);
        graph.add_edge(0, 1);
        graph.add_edge(0, 2);
        graph.add_edge(1, 3);
        graph.add_edge(2, 3);

        let mut result = compute_predecessors(&graph);
        result.values_mut().for_each(|v| v.sort());

        let expected: HashMap<NodeId, Vec<NodeId>> = HashMap::from([
            (0, vec![]),
            (1, vec![0]),
            (2, vec![0]),
            (3, vec![1, 2]),
        ]);
        assert_eq!(result, expected);
    }

    // A helper to create a HashSet from a slice for cleaner test code
    fn to_hashset(slice: &[NodeId]) -> HashSet<NodeId> {
        slice.iter().cloned().collect()
    }

    #[test]
    fn test_dominators_simple() {
        // Graph: 0 -> 1 -> 2
        let mut graph = TestGraph::new(0);
        graph.add_edge(0, 1);
        graph.add_edge(1, 2);

        let predecessors = compute_predecessors(&graph);
        let result = compute_dominators(&graph, &predecessors);

        let expected: HashMap<NodeId, HashSet<NodeId>> = HashMap::from([
            (0, to_hashset(&[0])),
            (1, to_hashset(&[0, 1])),
            (2, to_hashset(&[0, 1, 2])),
        ]);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_dominators_diamond() {
        // Graph:
        //   0
        //  / \
        // 1   2
        //  \ /
        //   3
        let mut graph = TestGraph::new(0);
        graph.add_edge(0, 1);
        graph.add_edge(0, 2);
        graph.add_edge(1, 3);
        graph.add_edge(2, 3);

        let predecessors = compute_predecessors(&graph);
        let result = compute_dominators(&graph, &predecessors);

        let expected: HashMap<NodeId, HashSet<NodeId>> = HashMap::from([
            (0, to_hashset(&[0])),
            (1, to_hashset(&[0, 1])),
            (2, to_hashset(&[0, 2])),
            (3, to_hashset(&[0, 3])), // Neither 1 nor 2 dominates 3
        ]);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_dominators_with_loop() {
        // Graph: 0 -> 1 <-> 2 -> 3
        let mut graph = TestGraph::new(0);
        graph.add_edge(0, 1);
        graph.add_edge(1, 2);
        graph.add_edge(2, 1); // Loop back
        graph.add_edge(2, 3);

        let predecessors = compute_predecessors(&graph);
        let result = compute_dominators(&graph, &predecessors);

        let expected: HashMap<NodeId, HashSet<NodeId>> = HashMap::from([
            (0, to_hashset(&[0])),
            (1, to_hashset(&[0, 1])), // To get to 1 or 2, you must pass 0 and 1
            (2, to_hashset(&[0, 1, 2])),
            (3, to_hashset(&[0, 1, 2, 3])),
        ]);

        assert_eq!(result, expected);
    }
}