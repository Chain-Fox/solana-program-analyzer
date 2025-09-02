use regex::Regex;
use rustc_public::mir::ProjectionElem;
use rustc_public::mir::StatementKind::Assign;
use rustc_public::mir::mono::Instance;
use rustc_public::mir::{AggregateKind, ConstOperand, Operand, Rvalue, TerminatorKind};
use rustc_public::ty::{AdtDef, AssocKind, FieldDef, MirConst, RigidTy, Ty, UintTy};
use rustc_public::{CompilerError, CrateDefItems};
use rustc_public::{CrateDef, CrateItem, ItemKind, run};
use std::ops::ControlFlow;
use std::process::ExitCode;

use rustc_public::Symbol;
use rustc_public::mir::StatementKind;
use rustc_public::ty::AdtKind;
use rustc_public::ty::Allocation;
use rustc_public::ty::ConstantKind::Allocated;
use rustc_public::ty::TyKind;
use rustc_public::ty::VariantDef;

/// Model an Anchor's account: #[account]
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

/// Model anchors' Accounts: #[derive(Accounts)]
#[derive(Debug)]
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

/// Collect all anchor Accounts defined locally by tracking trait anchor_lang::Accounts
pub fn local_anchor_accounts() -> Vec<AnchorAccounts> {
    let mut anchor_accounts_collection = vec![];
    let trait_impls = rustc_public::all_trait_impls();
    for trait_impl in trait_impls {
        let trait_name = trait_impl.trait_impl().value.def_id.name();
        // must be trait anchor_lang::Accounts
        if trait_name != ANCHOR_ACCOUNTS {
            continue;
        }
        // the type must be a local struct
        let self_ty = trait_impl.trait_impl().value.self_ty();
        if let Some(RigidTy::Adt(adt_def, _)) = self_ty.kind().rigid()
            && adt_def.krate().is_local
            && adt_def.kind() == AdtKind::Struct
        {
            for item in trait_impl.associated_items() {
                if let AssocKind::Fn { name, has_self } = item.kind
                    && name == "try_accounts"
                    && !has_self
                    && let Some(variant) = adt_def.variants_iter().next()
                    && let Some(anchor_accounts) = AnchorAccounts::from_variant(variant)
                {
                    anchor_accounts_collection.push(anchor_accounts);
                    break; // There can only be one `try_accounts` for one struct
                }
            }
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
        if !name.contains("__client_accounts") {
            // can also be "__cpi_client_accounts"
            continue;
        }
        let instance = match Instance::try_from(item) {
            Ok(instance) => instance,
            Err(_) => continue,
        };
        to_account_metas.push(instance);
    }
    let mut account_metas = vec![];
    for to_account_meta in to_account_metas {
        let body = match to_account_meta.body() {
            Some(body) => body,
            None => continue,
        };
        let first_arg_ty = if let Some(local_decl) = body.local_decl(1)  // first arg ty
            && let Some(RigidTy::Ref(_, next_ty, _)) = local_decl.ty.kind().rigid()
            && let Some(RigidTy::Adt(adt_def, _)) = next_ty.kind().rigid()
            && let Some(last) = adt_def.name().split(":").last()
        {
            last.to_owned()
        } else {
            continue;
        };
        for bb in body.blocks {
            if let TerminatorKind::Call {
                func,
                ..
            } = bb.terminator.kind
            && let Operand::Constant(const_operand) = func
            && let Some(RigidTy::FnDef(fn_def, _)) = const_operand.ty().kind().rigid()
            && (fn_def.name() == "anchor_lang::prelude::AccountMeta::new" || 
                fn_def.name() == "anchor_lang::prelude::AccountMeta::new_readonly")
            && let Some(statement) = bb.statements.last()  // the last statement (right before terminator)
            // Assign(_7, Use(Copy(((*_1).0: anchor_lang::prelude::Pubkey))))
            && let StatementKind::Assign(_, Rvalue::Use(Operand::Copy(ref place))) = statement.kind
            && place.local == 1  // The first arg
            && let [ProjectionElem::Deref, ProjectionElem::Field(field_idx, _)] = place.projection[..]
            {
                if fn_def.name() == "anchor_lang::prelude::AccountMeta::new" {
                    account_metas.push((first_arg_ty.clone(), "mut", field_idx));
                } else {
                    // new_readonly
                    account_metas.push((first_arg_ty.clone(), "immu", field_idx));
                }
            }
        }
    }
    account_metas
}

pub fn extract_program_id() -> Option<Vec<u8>> {
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

pub fn extract_discriminators() -> Vec<(String, Vec<u8>)> {
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
