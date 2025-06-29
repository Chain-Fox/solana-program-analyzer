use serde::Deserialize;
use std::collections::HashMap; // Import HashMap for parsing dependency tables
use std::fs;
use std::path::Path;
use thiserror::Error;
use toml::Value; // Import toml::Value to handle various dependency formats
use tracing::debug;

// Define a struct to mirror the structure of the Cargo.toml package section.
#[derive(Debug, Deserialize)]
struct Package {
    name: String,
}

// This struct will hold the extracted information for each dependency.
// Note: 'version' is an Option<String> because not all dependency entries
// directly specify a version (e.g., path dependencies or git dependencies).
#[derive(Debug)]
pub struct ParsedDependency {
    pub name: String,
    pub version: Option<String>,
}

// Define the main CargoToml struct for initial raw deserialization.
// We use HashMap<String, Value> for dependencies to handle different TOML formats.
#[derive(Debug, Deserialize)]
struct CargoTomlRaw {
    package: Package,
    #[serde(default)] // Use default to make this field optional in Cargo.toml
    dependencies: Option<HashMap<String, Value>>,
}

#[derive(Error, Debug)]
pub enum SolanaMetadataError {
    #[error("Cargo.toml not found")]
    CargoTomlNotFound,
    #[error("Cargo.toml fails to parse")]
    CargoTomlParseFailure,
}

pub fn parse_toml_in_crate_path(
    crate_path_str: &str,
) -> Result<(String, Vec<ParsedDependency>), SolanaMetadataError> {
    // Get the path to the Cargo.toml file from the environment variable PROGRAM_PATH.
    let crate_path = Path::new(&crate_path_str);
    let cargo_toml_path = crate_path.join("Cargo.toml");

    debug!("Attempting to parse: {}", cargo_toml_path.display());

    // Read the content of the Cargo.toml file.
    let toml_content = match fs::read_to_string(&cargo_toml_path) {
        Ok(content) => content,
        Err(_) => {
            return Err(SolanaMetadataError::CargoTomlNotFound);
        }
    };

    // Parse the TOML content into our CargoTomlRaw struct.
    let cargo_toml_raw: CargoTomlRaw = match toml::from_str(&toml_content) {
        Ok(parsed_toml) => parsed_toml,
        Err(_) => return Err(SolanaMetadataError::CargoTomlParseFailure),
    };

    // Extract the original package name.
    let original_name = cargo_toml_raw.package.name;

    // Convert the original package name to the crate name by replacing hyphens with underscores.
    let crate_name = original_name.replace('-', "_");

    // Process dependencies
    let mut parsed_dependencies: Vec<ParsedDependency> = Vec::new();
    if let Some(dependencies_map) = cargo_toml_raw.dependencies {
        for (dep_name, dep_value) in dependencies_map {
            let version = extract_version_from_toml_value(&dep_value);
            parsed_dependencies.push(ParsedDependency {
                name: dep_name,
                version,
            });
        }
    }

    Ok((crate_name, parsed_dependencies))
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum ProgramType {
    Anchor,
    SolanaNative,
    Other,
}

pub fn check_program_type(deps: &[ParsedDependency]) -> ProgramType {
    let mut program_type = ProgramType::Other;
    for dep in deps {
        if &dep.name == "anchor-lang" {
            program_type = ProgramType::Anchor;
            break;
        } else if (&dep.name == "solana-sdk" || &dep.name == "solana-program")
            && program_type == ProgramType::Other
        {
            program_type = ProgramType::SolanaNative;
        }
    }
    program_type
}

// Helper function to extract a version string from a toml::Value,
// which can be either a direct string or a table with a "version" key.
fn extract_version_from_toml_value(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()), // Direct version string
        Value::Table(table) => {
            if let Some(Value::String(s)) = table.get("version") {
                Some(s.clone()) // "version" key within a table
            } else {
                None // No "version" key in the table (e.g., path, git deps)
            }
        }
        _ => None, // Not a string or a table (uncommon for dependency values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_toml() {
        use std::env;
        let crate_path_str = env::var("SOLANA_PROGRAM").expect("SOLANA_PROGRAM not set");
        let (crate_name, parsed_dependencies) = parse_toml_in_crate_path(&crate_path_str).unwrap();
        // Print the results.
        println!("\n--- Result ---");
        println!("Crate name:  {}", crate_name);

        if !parsed_dependencies.is_empty() {
            println!("\n--- Dependencies ---");
            for dep in parsed_dependencies {
                match dep.version {
                    Some(v) => println!("- {}: {}", dep.name, v),
                    None => println!("- {}: (version not specified or complex)", dep.name),
                }
            }
        }
        println!("--------------");
    }
}
