use solana_program_analyzer::metadata::{check_program_type, parse_toml_in_crate_path};
use std::env;

fn main() {
    let crate_path_str = env::var("SOLANA_PROGRAM").expect("SOLANA_PROGRAM not set");
    let (crate_name, parsed_dependencies) = parse_toml_in_crate_path(&crate_path_str).unwrap();
    // Print the results.
    println!("\n--- Result ---");
    println!("Crate name:  {}", crate_name);

    if !parsed_dependencies.is_empty() {
        println!("\n--- Dependencies ---");
        for dep in parsed_dependencies.iter() {
            match &dep.version {
                Some(v) => println!("- {}: {}", dep.name, v),
                None => println!("- {}: (version not specified or complex)", dep.name),
            }
        }
    }
    println!("--------------");
    let program_type = check_program_type(&parsed_dependencies);
    println!("The type of the program is {:?}", program_type);
}
