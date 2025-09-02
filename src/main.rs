#![feature(rustc_private)]
#![feature(assert_matches)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_public;

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

    detect_duplicate_mutable_account();

    ControlFlow::Continue(())
}
