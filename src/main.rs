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

fn main() -> ExitCode {
    let rustc_args: Vec<_> = std::env::args().into_iter().collect();
    let result = run_with_tcx!(&rustc_args, solana_program_analyzer);
    match result {
        Ok(_) | Err(CompilerError::Skipped | CompilerError::Interrupted(_)) => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

fn solana_program_analyzer<'tcx>(_tcx: TyCtxt<'tcx>) -> ControlFlow<()> {
    let crate_name = stable_mir::local_crate().name;
    println!("{crate_name}");
    let local_items = stable_mir::all_local_items();
    for item in local_items {
        println!("{}", item.trimmed_name());
    }
    ControlFlow::Continue(())
}
