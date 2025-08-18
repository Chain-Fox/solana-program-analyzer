#![feature(rustc_private)]
#![feature(assert_matches)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_public;

use std::process::ExitCode;
use rustc_public::mir::mono::Instance;
use rustc_public::mir::{Operand, TerminatorKind};
use rustc_public::ty::{AdtDef, RigidTy};
use rustc_public::{run, CrateDef, CrateItem, ItemKind};
use rustc_public::{CompilerError};
use std::ops::ControlFlow;

mod analysis;

fn main() -> ExitCode {
    let rustc_args: Vec<_> = std::env::args().into_iter().collect();
    println!("{:?}", rustc_args);
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
    let entry_fn = rustc_public::entry_fn();
    println!("entry fn: {:?}", entry_fn);
    let body = entry_fn.unwrap().body().unwrap();
    // collect all uses of LockGuard
    // collect all def of LockGuard
    // for (local_idx, local_decl) in body.local_decls() {
    //     if let Some(rigid_ty) = local_decl.ty.kind().rigid() {
    //         // if let AdtDef
    //         match rigid_ty {
    //             RigidTy::Adt(adt_def, generic_args) => {
    //                 println!("Adt: {:?}", adt_def.name());
    //                 for generic_arg in &generic_args.0 {
    //                     println!("\t{:?}", generic_arg);
    //                 }
    //                 println!("");
    //             }
    //             _ => {}
    //         }
    //     }
    // }
    for block in body.blocks {
        match block.terminator.kind {
            TerminatorKind::Call { func, args, destination, target, unwind } => {
                match func {
                    Operand::Constant(op) => {
                        // println!("Call {:?}", op.ty().kind().fn_def()),
                        let kind = op.ty().kind();
                        let (def, generic_args) = kind.fn_def().unwrap();
                        println!("def: {:?}", def.name());
                        let instance = Instance::resolve(def, generic_args).unwrap();
                        println!("inst_def: {:?}", instance.def.name());
                    }
                    Operand::Copy(p) | Operand::Move(p) => println!("Call Place {:?}", p),
                }
            }
            _ => {
                println!("{:?}", block.terminator);
            }
        }
    }
    // for item in rustc_public::all_local_items() {
    //     match item.kind() {
    //         ItemKind::Fn => {
    //             println!("Fn {:?}", item);
    //         },
    //         ItemKind::Static => {
    //             println!("Static {:?}", item);
    //         },
    //         ItemKind::Const => {
    //             println!("Const {:?}", item);
    //         },
    //         ItemKind::Ctor(ctor_kind) => {
    //             println!("Ctor {:?}", ctor_kind);
    //         },
    //     }
    // }
    /*
    let external_crates = rustc_public::external_crates();
    for external_crate in external_crates {
        println!("external {}", external_crate.name);
        // println!("{:?}", external_crate.foreign_modules());
        // println!("{:?}", external_crate.fn_defs());
        println!("{:?}", external_crate.statics());
    }
    */

    ControlFlow::Continue(())
}