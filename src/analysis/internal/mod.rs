extern crate rustc_hir;

use rustc_middle::ty::TyCtxt;
use stable_mir::rustc_internal;
use stable_mir::DefId;

pub mod coercion;
pub mod reachability;

/// Return whether `def_id` refers to a nested static allocation.
pub fn is_anon_static(tcx: TyCtxt, def_id: DefId) -> bool {
    let int_def_id = rustc_internal::internal(tcx, def_id);
    match tcx.def_kind(int_def_id) {
        rustc_hir::def::DefKind::Static { nested, .. } => nested,
        _ => false,
    }
}
