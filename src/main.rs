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

use crate::analysis::account::local_anchor_accounts;

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
    for mono_item in mono_items {
        match mono_item {
            MonoItem::Fn(instance) => {
                let trimmed_name = instance.trimmed_name();
                if trimmed_name.contains("f32::<impl f32>::round")
                    || trimmed_name.contains("f64::<impl f64>::round")
                {
                    println!("{crate_name} contains f32::round or f64::round");
                }
            }
            _ => {}
        }
    }
    let anchor_accounts_collection = local_anchor_accounts();
    for anchor_accounts in anchor_accounts_collection {
        println!("{}", anchor_accounts.name);
        for (idx, anchor_account) in anchor_accounts.anchor_accounts.iter().enumerate() {
            println!("- {idx}: {:?}", &anchor_account);
        }
    }
    ControlFlow::Continue(())
}
