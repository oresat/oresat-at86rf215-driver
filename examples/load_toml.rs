use oresat_at86rf215_driver::radio::*;
use oresat_at86rf215_driver::config::*;
use toml;
use clap::Parser;
use std::fs::read_to_string;


#[derive(Parser)]
struct Args {
    file: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let contents = read_to_string(&args.file)?;
    let config: RadioConfig = toml::from_str(&contents)?;
    println!("load from file successful");

    let mut radio = Radio::new();
    radio.apply_config(&config);
    println!("apply_config successful");

    // Round-trip check: convert back to config and verify
    let roundtrip = radio.to_config();
    println!("to_config successful");
    let roundtrip_toml = toml::to_string(&roundtrip)?;
    let original_toml = toml::to_string(&config)?;

    if original_toml == roundtrip_toml {
        println!("OK: round-trip config matches");
    } else {
        eprintln!("Error: Round-trip config differs from input");
        eprintln!("--- original ---\n{}", original_toml);
        eprintln!("--- round-trip ---\n{}", roundtrip_toml);
        std::process::exit(1);
    }

    Ok(())
}
