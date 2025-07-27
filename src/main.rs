//! solana-program analyzer aims to be a full scale analysis tool for solana programs
//! The functions include but not limited to
//! 1. extract the Accounts
//! 2.
#![feature(rustc_private)]
#![feature(assert_matches)]
#![feature(let_chains)]

extern crate rustc_driver;
extern crate rustc_interface;
#[macro_use]
extern crate rustc_smir;
extern crate rustc_middle;
extern crate stable_mir;

// pub mod analysis;

use rustc_middle::ty::TyCtxt;
use rustc_smir::{run, rustc_internal};
use stable_mir::mir::mono::{Instance, MonoItem};
use stable_mir::mir::visit::Location;
use stable_mir::mir::{LocalDecl, MirVisitor, Operand, Terminator, TerminatorKind};
use stable_mir::ty::{AdtDef, AdtKind, AssocItemContainer, RigidTy, Ty, TyKind};
use stable_mir::{CompilerError, CrateDef, CrateDefItems, ItemKind};
use std::collections::HashSet;
use std::io::stdout;
use std::ops::ControlFlow;
use std::process::ExitCode;

use crate::analysis::account::{
    find_to_account_metas, local_anchor_accounts, AnchorAccount, AnchorAccountKind,
};

pub mod analysis;

fn main() -> ExitCode {
    let rustc_args: Vec<_> = std::env::args().into_iter().collect();
    let result = run_with_tcx!(&rustc_args, solana_program_analyzer);
    match result {
        Ok(_) | Err(CompilerError::Skipped | CompilerError::Interrupted(_)) => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

const ENTRY: &str = "entry";

/// Find the entry fn instance for solana program.
fn entry_fn() -> Option<Instance> {
    let crate_items = stable_mir::all_local_items();
    let mut entry_fn = None;
    for crate_item in crate_items {
        if crate_item.name() != ENTRY {
            continue;
        }
        if crate_item.requires_monomorphization() {
            continue;
        }
        let instance = match Instance::try_from(crate_item) {
            Ok(instance) => instance,
            Err(_) => continue,
        };
        entry_fn = Some(instance);
        break;
    }
    entry_fn
}

fn solana_program_analyzer<'tcx>(tcx: TyCtxt<'tcx>) -> ControlFlow<()> {
    let crate_name = stable_mir::local_crate().name;
    let target_crate_name = std::env::var("CRATE_NAME").unwrap_or("cfx_stake_core".to_owned());
    if target_crate_name != crate_name {
        return ControlFlow::Continue(());
    }

    let entry_fn = match stable_mir::entry_fn() {
        Some(entry_fn) => Instance::try_from(entry_fn).unwrap(),
        None => match entry_fn() {
            Some(entry_fn) => entry_fn,
            None => return ControlFlow::Continue(()),
        },
    };

    let local_reachable =
        analysis::internal::reachability::filter_crate_items(tcx, |_, instance| {
            let def_id = rustc_internal::internal(tcx, instance.def.def_id());
            instance == entry_fn || tcx.is_reachable_non_generic(def_id)
        })
        .into_iter()
        .map(MonoItem::Fn)
        .collect::<Vec<_>>();
    let mut transformer = analysis::internal::reachability::BodyTransformation {};
    let (mono_items, _) = analysis::internal::reachability::collect_reachable_items(
        tcx,
        &mut transformer,
        &local_reachable,
    );
    // for mono_item in mono_items {
    //     match mono_item {
    //         MonoItem::Fn(instance) => {
    //             let trimmed_name = instance.trimmed_name();
    //             if trimmed_name.contains("f32::<impl f32>::round")
    //                 || trimmed_name.contains("f64::<impl f64>::round")
    //             {
    //                 println!("{crate_name} contains f32::round or f64::round");
    //             }
    //         }
    //         _ => {}
    //     }
    // }

    //     to_account_metas(_1)

    // _x = ((*_1).0: anchor_lang::prelude::Pubkey);
    // _x1 = AccountMeta::new(move _11, true/false) -> [return: bb4, unwind: bb6];

    // _y = ((*_1).1: anchor_lang::prelude::Pubkey);
    // _y1 = AccountMeta::new(move _11, true/false) -> [return: bb4, unwind: bb6];

    // _1.0 data
    // _1.1 data
    // for mono_item in mono_items {
    //     match mono_item {
    //         MonoItem::Fn(instance) => {
    //             let trimmed_name = instance.trimmed_name();
    //             println!("{}", trimmed_name);
    //             if trimmed_name.contains("to_account_metas")
    //             {
    //                 println!("{trimmed_name}");
    //             }
    //         }
    //         _ => {}
    //     }
    // }

    let res = find_to_account_metas();
    // for r in res {
    //     println!("{:?}", r);
    // }

    let anchor_accounts_collection = local_anchor_accounts();
    for anchor_accounts in anchor_accounts_collection {
        println!("{}", anchor_accounts.name);
        let mut muts = vec![];
        for (name, mutability, field_idx) in res.iter() {
            if name == &anchor_accounts.name {
                muts.push((field_idx, mutability));
            }
        }
        let mut final_res = vec![];
        for (idx, anchor_account) in anchor_accounts.anchor_accounts.iter().enumerate() {
            let mut mu = None;
            for (field_idx, mutability) in muts.iter() {
                if *field_idx == &idx {
                    mu = Some(*mutability);
                    break;
                }
            }
            println!("- {idx}: {:?} {:?}", mu, &anchor_account);
            final_res.push((anchor_account, mu));
        }

        let len = final_res.len();
        for i in 0..len {
            for j in i + 1..len {
                if final_res[i].1 == Some(&"mut") && final_res[j].1 == Some(&"mut") {
                    match (final_res[i].0.kind.clone(), final_res[j].0.kind.clone()) {
                        (
                            AnchorAccountKind::Account(i_struct),
                            AnchorAccountKind::Account(j_struct),
                        ) if i_struct == j_struct => {
                            println!("Find error: two mutable accounts of the same type in the same Context: {:?} {:?}", final_res[i], final_res[j]);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    ControlFlow::Continue(())
}
