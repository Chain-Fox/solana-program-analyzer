use std::collections::HashSet;

use rustc_public::{mir::{mono::Instance, TerminatorKind}, ty::{RigidTy, TyKind}, ItemKind};

pub fn compute_instances() -> HashSet<Instance> {
    let mut local_instances = vec![];
    for item in rustc_public::all_local_items() {
        if let ItemKind::Fn = item.kind()
            && !item.requires_monomorphization()
            && let Ok(instance) = Instance::try_from(item) {
                local_instances.push(instance);
        }
    }
    // for instance in local_instances {
        // println!("{}", instance.name());
    // }

    let mut worklist = local_instances.clone();
    let mut nodes: HashSet<Instance> = local_instances.into_iter().collect();
    while let Some(curr) = worklist.pop() {
        if let Some(ref body) = curr.body() {
            for block in &body.blocks {
                if let TerminatorKind::Call {
                    ref func,
                    ..
                } = block.terminator.kind {
                    let fn_ty = func.ty(body.locals()).unwrap();
                    if let TyKind::RigidTy(RigidTy::FnDef(fn_def, args)) = fn_ty.kind() {
                        let instance = Instance::resolve(fn_def, &args).unwrap();
                        if nodes.insert(instance) {
                            worklist.push(instance);
                        }
                    }
                }
            }
        }
    }

    return nodes
}