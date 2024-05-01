use clap::Parser;
use log::error;
use std::{fs, path::Path, process::exit};

/// A package builder for Some OS
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Directory with SOMEBUILD.toml
    #[arg(short, long)]
    input: String,

    /// Building directory
    #[arg(short, long)]
    output: String,
}

fn main() {
    env_logger::builder()
        .format_timestamp(None)
        .format_target(false)
        .init();

    let args = Args::parse();

    let input = Path::new(&args.input);
    let output = Path::new(&args.output);

    if !input.exists() {
        error!("Input path \"{}\" does not exist!", args.input);
        exit(1);
    }
    if !output.exists() {
        error!("Output path \"{}\" does not exist!", args.output);
        exit(1);
    }

    let input = fs::canonicalize(&input).unwrap();
    let output = fs::canonicalize(&output).unwrap();

    if input.is_file() {
        error!("Input is a file not a directory!");
        exit(1);
    }
    if output.is_file() {
        error!("Output is a file not a directory!");
        exit(1);
    }

    println!("Input dir: {:?}",  input);
    println!("Output dir: {:?}", output);
}
