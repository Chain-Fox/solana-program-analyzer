use stable_mir::{
    ty::{FieldDef, RigidTy, TyKind, AdtKind, VariantDef},
    CrateDef, DefId,
};

pub struct AnchorAccounts {
    def_id: DefId,
}

const ANCHOR_ACCOUNTS: &str = "anchor_lang::Accounts<";

pub fn is_anchor_accounts(ty_name: &str) -> bool {
    ty_name.contains(ANCHOR_ACCOUNTS)
}

impl AnchorAccounts {}

pub enum AnchorAccountKind {
    Account(TyKind),
    Signer,
    Program,
    Sysvar(TyKind),
}

impl AnchorAccountKind {
    pub fn from_ty(kind: &TyKind) -> Option<Self> {
        if let RigidTy::Adt(adt_def, generics) = kind.rigid()? {
            match adt_def.name().as_ref() {
                "anchor_lang::prelude::Account" => {
                    // e.g.
                    // RigidTy(Adt(AdtDef(DefId { id: 452, name: "anchor_lang::prelude::Account" }), GenericArgs([Lifetime(Region { kind: ReEarlyParam(EarlyParamRegion { index: 0, name: "'info" }) }), Type(Ty { id: 111, kind: RigidTy(Adt(AdtDef(DefId { id: 42649, name: "StakePool" }), GenericArgs([]))) })])))
                    let concrete_ty = generics.0.get(1)?.ty()?;
                    Some(Self::Account(concrete_ty.kind()))
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
                    let concrete_ty = generics.0.get(1)?.ty()?;
                    Some(Self::Sysvar(concrete_ty.kind()))
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

pub struct AnchorAccount {
    pub name: String,
    pub kind: AnchorAccountKind,
}

// impl AnchorAccount {
//     pub fn from_variant(field_def: &FieldDef) -> Option<Self> {
//         let name = field_def.name;
//         let kind = field_def.ty();
//         match kind {

//         }
//         todo!{}
//     }
// }

// fn test() {
//     let trait_impls = stable_mir::all_trait_impls();
//     for trait_impl in trait_impls {
//         let self_ty = trait_impl.trait_impl().value.self_ty();
//         match self_ty.kind() {
//             TyKind::RigidTy(RigidTy::Adt(adt_def, generic_args))
//                 if adt_def.kind() == AdtKind::Struct =>
//             {
//                 for item in trait_impl.associated_items() {
//                     match item.kind {
//                         stable_mir::ty::AssocKind::Fn { name, has_self } => {
//                             if name == "try_accounts" && !has_self {
//                                 // resolve instance from assoc_fn
//                                 // println!("item.def_id: {:?}, {}", item.def_id, has_self);
//                                 // println!("{:?}", adt_def);
//                                 // println!("{:?}", adt_def);
//                                 if let Some(variant) = adt_def.variants_iter().next() {
//                                     // println!("{:?}", variant);
//                                     // println!("{:?}", variant.fields());
//                                     println!("{:?}", variant.name());
//                                     for field in variant.fields() {
//                                         println!("{:?}, {:?}", field.name, field.ty().kind());
//                                     }
//                                 } else {
//                                     eprintln!("Error");
//                                 }
//                                 // println!("{:?}", generic_args);
//                                 // try_account_fn_ids.push(item.def_id.def_id());
//                             }
//                         }
//                         _ => {}
//                     }
//                 }
//             }
//             _ => {}
//         }
//     }
// }
