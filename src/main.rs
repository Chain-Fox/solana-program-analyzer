#![feature(rustc_private)]
#![feature(assert_matches)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_public;

use rustc_public::mir::Body;
use rustc_public::CompilerError;
use rustc_public::run;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::ControlFlow;
use std::process::ExitCode;

use crate::anchor_info::entry_instance;
use rustc_public::CompilerError;
use rustc_public::run;
use std::ops::ControlFlow;
use std::process::ExitCode;

use crate::anchor_info::{extract_discriminators, extract_program_id};
use crate::checker::detect_duplicate_mutable_account;

mod analysis;
mod anchor_info;
mod checker;

fn main() -> ExitCode {
    let rustc_args: Vec<_> = std::env::args().collect();
    let result = run!(&rustc_args, demo_analysis);
    match result {
        Ok(_) | Err(CompilerError::Skipped | CompilerError::Interrupted(_)) => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

fn demo_analysis() -> ControlFlow<()> {
    println!("Analyzing");
    let local_crate = rustc_public::local_crate();
    println!("crate: {}", local_crate.name);
    if local_crate.name != "cfx_stake_core" {
        return ControlFlow::Continue(());
    }

    let program_id = extract_program_id();
    println!("{:?}", program_id);

    let discriminators = extract_discriminators();
    println!("{:?}", discriminators);

    if let Some(entry) = entry_instance()
        && let Some(body) = entry.body()
    {
        
        let preds = compute_preds(&body);
        println!("{:?}", preds);

        let dominators = compute_dominators(&body, &preds);
        println!("{:?}", dominators);

        let post_dominators = compute_postdominators(&body);
        println!("{:?}", post_dominators);
    }

    detect_duplicate_mutable_account();

    ControlFlow::Continue(())
}

fn compute_preds(body: &Body) -> HashMap<usize, HashSet<usize>> {
    let mut preds: HashMap<usize, HashSet<usize>> = HashMap::new();
    let mut worklist: Vec<usize> = (0..body.blocks.len()).collect();

    while let Some(bb) = worklist.pop() {
        // Get the successors of the current block.
        let succs = body.blocks[bb].terminator.successors();

        for succ in succs {
            let pred_set = preds.entry(succ).or_default();

            if pred_set.insert(bb) {
                // If a new predecessor was found for `succ`,
                // add `succ` to the worklist to propagate the information.
                worklist.push(succ);
            }
        }
    }
    preds
}

fn compute_dominators(body: &Body, preds: &HashMap<usize, HashSet<usize>>) -> HashMap<usize, HashSet<usize>> {
    let mut doms: HashMap<usize, HashSet<usize>> = HashMap::new();
    let num_blocks = body.blocks.len();

    // The entry block (block 0) dominates itself.
    let mut entry_dom_set = HashSet::new();
    entry_dom_set.insert(0);
    doms.insert(0, entry_dom_set);

    // All other nodes initially have a dominator set containing all nodes.
    for i in 1..num_blocks {
        let all_blocks: HashSet<usize> = (0..num_blocks).collect();
        doms.insert(i, all_blocks);
    }

    let mut changed = true;
    while changed {
        changed = false;
        // The algorithm iterates until there are no changes to the dominator sets.
        for i in 1..num_blocks {
            if let Some(predecessors) = preds.get(&i) {
                // Intersect the dominator sets of all predecessors.
                let mut intersection = (0..num_blocks).collect::<HashSet<usize>>();
                
                let mut first_pred = true;
                for &p in predecessors {
                    if let Some(pred_doms) = doms.get(&p) {
                        if first_pred {
                            intersection = pred_doms.clone();
                            first_pred = false;
                        } else {
                            intersection = &intersection & pred_doms;
                        }
                    }
                }
                
                // Add the current block to its own dominator set.
                intersection.insert(i);

                if let Some(current_doms) = doms.get_mut(&i) {
                    if *current_doms != intersection {
                        *current_doms = intersection;
                        changed = true;
                    }
                }
            }
        }
    }
    doms
}

fn compute_postdominators(body: &Body) -> HashMap<usize, HashSet<usize>> {
    let mut postdoms: HashMap<usize, HashSet<usize>> = HashMap::new();
    let num_blocks = body.blocks.len();
    let mut exit_nodes = HashSet::new();
    
    // Find all exit nodes (blocks with no successors).
    for i in 0..num_blocks {
        if body.blocks[i].terminator.successors().is_empty() {
            exit_nodes.insert(i);
        }
    }

    // Initialize post-dominator sets.
    for i in 0..num_blocks {
        if exit_nodes.contains(&i) {
            let mut pd_set = HashSet::new();
            pd_set.insert(i);
            postdoms.insert(i, pd_set);
        } else {
            let all_blocks: HashSet<usize> = (0..num_blocks).collect();
            postdoms.insert(i, all_blocks);
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        // The algorithm iterates until there are no changes to the post-dominator sets.
        // We iterate over all nodes except the exit nodes.
        for i in (0..num_blocks).rev() { // Iterating in reverse can improve performance but is not required for correctness.
            if !exit_nodes.contains(&i) {
                let succs = body.blocks[i].terminator.successors();
                
                // Intersect the post-dominator sets of all successors.
                let mut intersection = (0..num_blocks).collect::<HashSet<usize>>();
                
                let mut first_succ = true;
                for s in succs {
                    if let Some(succ_pds) = postdoms.get(&s) {
                        if first_succ {
                            intersection = succ_pds.clone();
                            first_succ = false;
                        } else {
                            intersection = &intersection & succ_pds;
                        }
                    }
                }
                
                // Add the current block to its own post-dominator set.
                intersection.insert(i);

                if let Some(current_pds) = postdoms.get_mut(&i) {
                    if *current_pds != intersection {
                        *current_pds = intersection;
                        changed = true;
                    }
                }
            }
        }
    }
    postdoms
}
