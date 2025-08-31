#![feature(rustc_private)]
#![feature(assert_matches)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_public;

use regex::Regex;
use rustc_public::{CompilerError, CrateDefItems};
use rustc_public::mir::StatementKind::Assign;
use rustc_public::mir::mono::Instance;
use rustc_public::mir::{AggregateKind, ConstOperand, Operand, Rvalue, TerminatorKind};
use rustc_public::ty::{AdtDef, FieldDef, MirConst, RigidTy, Ty, UintTy};
use rustc_public::{CrateDef, CrateItem, ItemKind, run};
use std::ops::ControlFlow;
use std::process::ExitCode;
use rustc_public::mir::ProjectionElem;

use rustc_public::ty::Allocation;
use rustc_public::ty::ConstantKind::Allocated;
use rustc_public::mir::StatementKind;
use rustc_public::ty::AdtKind;
use rustc_public::Symbol;
use rustc_public::ty::TyKind;
use rustc_public::ty::VariantDef;

mod analysis;

    
fn main() -> ExitCode {
    let rustc_args: Vec<_> = std::env::args().into_iter().collect();
    // println!("{:?}", rustc_args);
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

    let res = find_to_account_metas();
    println!("{:?}", res);
    // let entry_fn = rustc_public::entry_fn();
    // println!("entry fn: {:?}", entry_fn);

    // for item in rustc_public::all_local_items() {
    //     match item.kind() {
    //         ItemKind::Fn => {
    //             // println!("Fn {:?}", item);
    //         },
    //         ItemKind::Static => {
    //             // println!("static {:?}", item);
    //             if item.name() == "ID" {
    //                 for stmt in &item.body().unwrap().blocks[0].statements {
    //                     match &stmt.kind {
    //                         Assign(place, rvalue) => {
    //                             // println!("{:?}", rvalue);
    //                             if let Rvalue::Aggregate(rustc_public::mir::AggregateKind::Array(ty), operands) = rvalue {
    //                                 if let Some(elem_ty) = ty.kind().rigid() {
    //                                     if let RigidTy::Uint(inner) = elem_ty {
    //                                         if let UintTy::U8 = inner {
    //                                             let mut id = Vec::with_capacity(operands.len());
    //                                             for operand in operands {
    //                                                 match operand {
    //                                                     Operand::Constant(ConstOperand { span, user_ty, const_}) => {
    //                                                         // println!("{:?}", const_.kind());
    //                                                         match const_.kind() {
    //                                                             Allocated(Allocation { bytes, ..}) => {
    //                                                                 for byte in bytes {
    //                                                                     if let Some(byte) = byte {
    //                                                                         id.push(*byte);
    //                                                                     } else {
    //                                                                         break;
    //                                                                     }
    //                                                                 }

    //                                                             }
    //                                                             _ => {}
    //                                                         }
    //                                                     }
    //                                                     _ => {}
    //                                                 }
    //                                             }
    //                                             println!("{:?}", id);
    //                                             break;
    //                                         }
    //                                     }
    //                                 }
    //                             }
    //                         }
    //                         _ => {}
    //                     }

    //                 }
    //                 // the first block
    //                 // the first assignment

    //             }
    //         },
    //         ItemKind::Const => {
    //             // println!("const {:?}", item);
    //         }
    //         ItemKind::Ctor(_) => {
    //             // println!("ctor {:?}", item);
    //         }
    //     }
    // }

    fn extract_program_id() -> Option<Vec<u8>> {
        let mut program_id = None;
        for item in rustc_public::all_local_items() {
            if !matches!(item.kind(), ItemKind::Static) {
                continue;
            }

            if item.name() != "ID" {
                continue;
            }

            let body = match item.body() {
                Some(b) => b,
                None => continue,
            };

            // look at the first block's statements
            for stmt in &body.blocks[0].statements {
                let (_, rvalue) = match &stmt.kind {
                    Assign(place, rvalue) => (place, rvalue),
                    _ => continue,
                };

                // array of u8 check
                let (ty, operands) = match rvalue {
                    Rvalue::Aggregate(AggregateKind::Array(ty), operands) => (ty, operands),
                    _ => continue,
                };

                let RigidTy::Uint(UintTy::U8) = ty.kind().rigid()? else {
                    continue;
                };

                let mut id = Vec::with_capacity(operands.len());
                for operand in operands {
                    if let Operand::Constant(ConstOperand { const_, .. }) = operand
                        && let Allocated(Allocation { bytes, .. }) = const_.kind()
                    {
                        for byte in bytes.iter().flatten() {
                            id.push(*byte);
                        }
                    }
                }

                program_id = Some(id);
                return program_id;
            }
        }
        program_id
    }

    fn extract_discriminators() -> Vec<(String, Vec<u8>)> {
        let re = Regex::new(r"<(.+?)\s+as\s+anchor_lang::Discriminator>").unwrap();
        let mut account_discriminators = vec![];
        for item in rustc_public::all_local_items() {
            if !matches!(item.kind(), ItemKind::Const) {
                continue;
            }

            let item_name = item.name();

            if !item_name.ends_with("::DISCRIMINATOR") {
                continue;
            }

            if item_name.starts_with("<instruction::") {
                continue;
            }

            let account_name = if let Some(caps) = re.captures(&item_name) {
                let account = &caps[1];
                account.to_owned()
            } else {
                continue;
            };

            let body = match item.body() {
                Some(b) => b,
                None => continue,
            };

            for stmt in &body.blocks[0].statements {
                let (_, rvalue) = match &stmt.kind {
                    Assign(place, rvalue) => (place, rvalue),
                    _ => continue,
                };

                // array of u8 check
                let (ty, operands) = match rvalue {
                    Rvalue::Aggregate(AggregateKind::Array(ty), operands) => (ty, operands),
                    _ => continue,
                };

                let RigidTy::Uint(UintTy::U8) = ty.kind().rigid().unwrap() else {
                    continue;
                };

                let mut id = Vec::with_capacity(operands.len());
                for operand in operands {
                    if let Operand::Constant(ConstOperand { const_, .. }) = operand
                        && let Allocated(Allocation { bytes, .. }) = const_.kind()
                    {
                        for byte in bytes.iter().flatten() {
                            id.push(*byte);
                        }
                    }
                }

                account_discriminators.push((account_name, id));
                break;
                
            }
        }
        account_discriminators
    }

    // // let body = entry_fn.unwrap().body().unwrap();
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
    // for block in body.blocks {
    //     match block.terminator.kind {
    //         TerminatorKind::Call { func, args, destination, target, unwind } => {
    //             match func {
    //                 Operand::Constant(op) => {
    //                     // println!("Call {:?}", op.ty().kind().fn_def()),
    //                     let kind = op.ty().kind();
    //                     let (def, generic_args) = kind.fn_def().unwrap();
    //                     println!("def: {:?}", def.name());
    //                     let instance = Instance::resolve(def, generic_args).unwrap();
    //                     println!("inst_def: {:?}", instance.def.name());
    //                 }
    //                 Operand::Copy(p) | Operand::Move(p) => println!("Call Place {:?}", p),
    //             }
    //         }
    //         _ => {
    //             println!("{:?}", block.terminator);
    //         }
    //     }
    // }
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



pub struct AnchorAccounts {
    pub name: String,
    pub anchor_accounts: Vec<AnchorAccount>,
}

pub const ANCHOR_ACCOUNTS: &str = "anchor_lang::Accounts";
pub const TO_ACCOUNT_METAS: &str = "to_account_metas";

impl AnchorAccounts {
    pub fn from_variant(variant: VariantDef) -> Option<Self> {
        let fields = variant.fields();
        let mut anchor_accounts = Vec::with_capacity(fields.len());
        for field_def in fields {
            if let Some(anchor_account) = AnchorAccount::from_field_def(&field_def) {
                anchor_accounts.push(anchor_account);
            }
        }
        Some(Self {
            name: variant.name(),
            anchor_accounts,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnchorAccountKind {
    Account(Symbol),
    Signer,
    Program,
    Sysvar(Symbol),
}

impl AnchorAccountKind {
    pub fn from_ty(kind: &TyKind) -> Option<Self> {
        if let RigidTy::Adt(adt_def, generics) = kind.rigid()? {
            match adt_def.name().as_ref() {
                "anchor_lang::prelude::Account" => {
                    // e.g.
                    // RigidTy(Adt(AdtDef(DefId { id: 452, name: "anchor_lang::prelude::Account" }), GenericArgs([Lifetime(Region { kind: ReEarlyParam(EarlyParamRegion { index: 0, name: "'info" }) }), Type(Ty { id: 111, kind: RigidTy(Adt(AdtDef(DefId { id: 42649, name: "StakePool" }), GenericArgs([]))) })])))
                    if let RigidTy::Adt(adt_def, _) = generics.0.get(1)?.ty()?.kind().rigid()? {
                        Some(Self::Account(adt_def.name()))
                    } else {
                        None
                    }
                }
                "anchor_lang::prelude::Signer" => {
                    // e.g.
                    // "authority", RigidTy(Adt(AdtDef(DefId { id: 454, name: "anchor_lang::prelude::Signer" }), GenericArgs([Lifetime(Region { kind: ReEarlyParam(EarlyParamRegion { index: 0, name: "'info" }) })])))
                    Some(Self::Signer)
                }
                "anchor_lang::prelude::Program" => {
                    // e.g.
                    // "system_program", RigidTy(Adt(AdtDef(DefId { id: 460, name: "anchor_lang::prelude::Program" }), GenericArgs([Lifetime(Region { kind: ReEarlyParam(EarlyParamRegion { index: 0, name: "'info" }) }), Type(Ty { id: 131, kind: RigidTy(Adt(AdtDef(DefId { id: 42667, name: "anchor_lang::system_program::System" }), GenericArgs([]))) })])))
                    Some(Self::Program)
                }
                "anchor_lang::prelude::Sysvar" => {
                    // e.g.
                    // "rent", RigidTy(Adt(AdtDef(DefId { id: 459, name: "anchor_lang::prelude::Sysvar" }), GenericArgs([Lifetime(Region { kind: ReEarlyParam(EarlyParamRegion { index: 0, name: "'info" }) }), Type(Ty { id: 129, kind: RigidTy(Adt(AdtDef(DefId { id: 579, name: "anchor_lang::prelude::Rent" }), GenericArgs([]))) })])))
                    if let RigidTy::Adt(adt_def, _) = generics.0.get(1)?.ty()?.kind().rigid()? {
                        Some(Self::Account(adt_def.name()))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnchorAccount {
    pub name: String,
    pub kind: AnchorAccountKind,
}

impl AnchorAccount {
    pub fn from_field_def(field_def: &FieldDef) -> Option<Self> {
        let kind = field_def.ty().kind();
        let anchor_account_kind = AnchorAccountKind::from_ty(&kind)?;
        Some(Self {
            name: field_def.name.clone(),
            kind: anchor_account_kind,
        })
    }
}

pub fn local_anchor_accounts() -> Vec<AnchorAccounts> {
    let mut anchor_accounts_collection = vec![];
    let trait_impls = rustc_public::all_trait_impls();
    for trait_impl in trait_impls {
        let trait_name = trait_impl.trait_impl().value.def_id.name();
        if trait_name != ANCHOR_ACCOUNTS {
            continue;
        }
        let self_ty = trait_impl.trait_impl().value.self_ty();
        match self_ty.kind().rigid() {
            Some(RigidTy::Adt(adt_def, _))
                if adt_def.krate().is_local && adt_def.kind() == AdtKind::Struct =>
            {
                for item in trait_impl.associated_items() {
                    match item.kind {
                        rustc_public::ty::AssocKind::Fn { name, has_self } => {
                            if name == "try_accounts" && !has_self {
                                if let Some(variant) = adt_def.variants_iter().next() {
                                    if let Some(anchor_accounts) =
                                        AnchorAccounts::from_variant(variant)
                                    {
                                        anchor_accounts_collection.push(anchor_accounts);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    anchor_accounts_collection
}


pub fn find_to_account_metas() -> Vec<(String, &'static str, usize)> {
    let mut to_account_metas = vec![];
    let items = rustc_public::all_local_items();
    for item in items {
        let name = item.name();
        if !name.contains(TO_ACCOUNT_METAS) {
            continue;
        }
        if !name.contains(&"__client_accounts") {
            continue;
        }
        // if name.contains(&"__cpi_client_accounts") {
        //     continue;
        // }
        let instance = match Instance::try_from(item) {
            Ok(instance) => instance,
            Err(_) => continue,
        };
        to_account_metas.push(instance);
    }
    // println!("{:?}", to_account_metas);
    let mut account_metas = vec![];
    for to_account_meta in to_account_metas {
        let body = match to_account_meta.body() {
            Some(body) => body,
            None => continue,
        };
        let first_arg_ty = match &body.local_decl(1) {
            // Ty { id: 889, kind: RigidTy(Ref(Region { kind: ReErased }, Ty { id: 891, kind: RigidTy(Adt(AdtDef(DefId { id: 353, name: "distribute::__client_accounts_distribute_rewards::DistributeRewards" }), GenericArgs([]))) }, Not)) }
            Some(local_decl) => {
                match local_decl.ty.kind().rigid() {
                    Some(RigidTy::Ref(region, next_ty, mutability)) => {
                        // println!("{:?}", next_ty);

                        match next_ty.kind().rigid() {
                            Some(RigidTy::Adt(adt_def, _)) => {
                                // println!("{:?}", adt_def.name);
                                let name = adt_def.name();
                                let fields = name.split(":");
                                if let Some(last) = fields.last() {
                                    last.to_string()
                                } else {
                                    continue;
                                }
                            }
                            _ => continue,
                        }
                        // next_ty.kind().rigid().unwrap
                    }
                    _ => continue,
                }
            }
            None => continue,
        };
        println!("{:?}", first_arg_ty);
        // body.locals[1].ty()
        for bb in body.blocks {
            // println!("{:?}", bb.terminator);
            match bb.terminator.kind {
                TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    target,
                    unwind,
                } => {
                    match func {
                        Operand::Constant(const_operand) => {
                            // println!("{:?}", const_operand.ty());
                            // Ty { id: 887, kind: RigidTy(FnDef(FnDef(DefId { id: 355, name: "anchor_lang::prelude::AccountMeta::new" }), GenericArgs([]))) }
                            if let Some(RigidTy::FnDef(fn_def, _)) =
                                const_operand.ty().kind().rigid()
                            {
                                if fn_def.name() == "anchor_lang::prelude::AccountMeta::new"
                                    || fn_def.name()
                                        == "anchor_lang::prelude::AccountMeta::new_readonly"
                                {
                                    // println!("{:?}", fn_def);
                                    match bb.statements.last() {
                                        Some(statement) => {
                                            // println!("{:?}", statement);
                                            match statement.kind {
                                                // Assign(_7, Use(Copy(((*_1).0: anchor_lang::prelude::Pubkey))))
                                                StatementKind::Assign(
                                                    _,
                                                    Rvalue::Use(Operand::Copy(ref place)),
                                                ) => {
                                                    // println!("{place:?}");
                                                    // check place ty
                                                    if place.local == 1 {
                                                        // println!("{:?}", place.projection);
                                                        if let [ProjectionElem::Deref, ProjectionElem::Field(field_idx, _)] =
                                                            place.projection[..]
                                                        {
                                                            println!("{:?}", field_idx);
                                                            if fn_def.name() == "anchor_lang::prelude::AccountMeta::new" {
                                                                account_metas.push((first_arg_ty.clone(), "mut", field_idx));
                                                            } else {
                                                                account_metas.push((first_arg_ty.clone(), "immu", field_idx));
                                                            }
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                        None => {}
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    // anchor_lang::prelude::AccountMeta::new
                }
                _ => {}
            }
        }
    }
    account_metas
}