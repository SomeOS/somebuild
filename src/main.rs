mod somebuild_config;

use clap::Parser;
use futures::{io::BufReader, TryStreamExt};
use log::error;
use std::{
    fs::{self, File},
    io::{Error, ErrorKind, Read},
    path::Path,
    process::exit,
};

use crate::somebuild_config::Config;

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

#[tokio::main]
async fn main() {
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

    let input = fs::canonicalize(input).unwrap();
    let output = fs::canonicalize(output).unwrap();

    if input.is_file() {
        error!("Input is a file not a directory!");
        exit(1);
    }
    if output.is_file() {
        error!("Output is a file not a directory!");
        exit(1);
    }

    println!("Input dir:\t{:?}", input);
    println!("Output dir:\t{:?}", output);

    let mut somebuild_file =
        File::open(input.join("SOMEBUILD.toml")).expect("Failed to open SOMEBUILD.toml!");
    let mut config_str = String::new();

    somebuild_file
        .read_to_string(&mut config_str)
        .expect("Failed to read SOMEBUILD.toml!");

    let config: Config = toml::from_str(&config_str).expect("Failed to parse SOMEBUILD.toml!");

    println!(
        "Package:\t{}-{}_{}",
        config.general.name, config.source.version, config.source.release
    );

    let response = reqwest::get(config.source.url).await.unwrap();

    let reader = response
        .bytes_stream()
        .map_err(|e| Error::new(ErrorKind::Other, e))
        .into_async_read();

    let decoder = async_compression::futures::bufread::ZstdDecoder::new(BufReader::new(reader));

    let archive = async_tar::Archive::new(decoder);

    let _ = archive.unpack(output).await;
}
