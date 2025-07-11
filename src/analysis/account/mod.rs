use stable_mir::{
    ty::{AdtKind, EarlyBinder, FieldDef, RigidTy, TyKind, VariantDef},
    CrateDef, CrateDefItems, Symbol,
};

pub struct AnchorAccounts {
    pub name: String,
    pub anchor_accounts: Vec<AnchorAccount>,
}

pub const ANCHOR_ACCOUNTS: &str = "anchor_lang::Accounts";

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
    let trait_impls = stable_mir::all_trait_impls();
    for trait_impl in trait_impls {
        let trait_name = trait_impl.trait_impl().value.def_id.name();
        if trait_name != ANCHOR_ACCOUNTS {
            continue
        }
        let self_ty = trait_impl.trait_impl().value.self_ty();
        match self_ty.kind().rigid() {
            Some(RigidTy::Adt(adt_def, _))
                if adt_def.krate().is_local && adt_def.kind() == AdtKind::Struct =>
            {
                for item in trait_impl.associated_items() {
                    match item.kind {
                        stable_mir::ty::AssocKind::Fn { name, has_self } => {
                            if name == "try_accounts" && !has_self {
                                // println!("{:?}", trait_impl.trait_impl());
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