//! Find meta info for Programs written in native Rust or Anchor frameworks.
//! 1. Decide if the curren package is Solana/Anchor/Not.
//! 2. Get the package/library name and the dep versions of solana-sdk/Anchor.

pub mod parser;
pub mod vulnerability;
pub use parser::{
    check_program_type, parse_toml_in_crate_path, ParsedDependency, ProgramType,
    SolanaMetadataError,
};
pub use vulnerability::detect_vulnerable_dep;
