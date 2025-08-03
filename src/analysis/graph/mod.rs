use stable_mir::mir::{Body, BasicBlockIdx};
use std::{collections::HashMap};

mod directed_graph;

pub use directed_graph::{DirectedGraph, compute_predecessors};

pub struct ControlFlowGraphAnalysis<'a> {
    body: &'a Body,
    successors: HashMap<BasicBlockIdx, Vec<BasicBlockIdx>>,
    predecessors: Option<HashMap<BasicBlockIdx, Vec<BasicBlockIdx>>>,
    // dominators
    // post_dominators
}

impl<'a> ControlFlowGraphAnalysis<'a> {
    pub fn new(body: &'a Body) -> Self {
        let mut successors = HashMap::new();
        for (idx, block) in body.blocks.iter().enumerate() {
            successors.insert(idx, block.terminator.successors());
        }
        Self {
            body,
            successors,
            predecessors: None,
        }
    }

    pub fn predecessors(&mut self) -> &HashMap<BasicBlockIdx, Vec<BasicBlockIdx>> {
        if self.predecessors.is_none() {
            let preds = compute_predecessors(self);
            self.predecessors = Some(preds);
        }

        // Now predecessors must be Some. So unwrap here will not panic.
        self.predecessors.as_ref().unwrap()
    }
}

impl<'a> DirectedGraph for ControlFlowGraphAnalysis<'a> {
    type Item = BasicBlockIdx;

    fn nodes(&self) -> impl Iterator<Item = Self::Item> {
        0..self.body.blocks.len()
    }

    fn start_node(&self) -> Self::Item {
        0
    }

    fn successors(&self) -> &HashMap<Self::Item, Vec<Self::Item>> {
        &self.successors
    }
}

