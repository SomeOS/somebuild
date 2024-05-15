mod bars;
mod command;
mod decompress;
mod downloader;
mod log;
mod paths;
mod somebuild_config;

use clap::Parser;
use indicatif::ProgressBar;
use tokio::io::AsyncReadExt;

use std::path::Path;
use tokio::fs::{self, File};

use crate::bars::{create_multibar, ProgressStyle};
use crate::downloader::Download;
use crate::log::*;
use crate::paths::normalize_path;
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
    let multibar = create_multibar();

    let args = Args::parse();

    let input = Path::new(&args.input);
    let output = Path::new(&args.output);

    let input = match fs::canonicalize(input).await {
        Ok(input) => input,
        Err(error) => {
            fatal!(
                "Failed reading input path: \"{}\"\n\t with error \"{}\"",
                normalize_path(input).display(),
                error
            );
        }
    };
    let output = match fs::canonicalize(output).await {
        Ok(output) => output,
        Err(error) => {
            fatal!(
                "Failed reading output path: \"{}\"\n\t with error \"{}\"",
                normalize_path(output).display(),
                error
            );
        }
    };

    if input.is_file() {
        fatal!("Input is a file not a directory!");
    }
    if output.is_file() {
        fatal!("Output is a file not a directory!");
    }

    info!("Input dir:\t{:?}", input);
    info!("Output dir:\t{:?}", output);

    let mut somebuild_file = File::open(input.join("SOMEBUILD.toml"))
        .await
        .expect("Failed to open SOMEBUILD.toml!");
    let mut config_str = String::new();

    somebuild_file
        .read_to_string(&mut config_str)
        .await
        .expect("Failed to read SOMEBUILD.toml!");

    let config: Config = toml::from_str(&config_str).expect("Failed to parse SOMEBUILD.toml!");

    info!("Download Url:\t{}", config.source.url);

    info!(
        "Package:\t{}-{}_{}",
        config.general.name, config.source.version, config.source.release
    );

    let down = Download::new(&multibar, &config, &output);

    down.download().await;

    let bar_build = multibar.add(ProgressBar::new(3));
    bar_build.set_style(ProgressStyle::Build.value());
    bar_build.tick();
    down.finish();

    bar_build.set_message(format!(
        "Setup {}-{}",
        config.general.name, config.source.version
    ));
    command::run(&config.build.setup, output.join(&down.file_name));
    bar_build.inc(1);

    bar_build.set_message(format!(
        "Building {}-{}",
        config.general.name, config.source.version
    ));
    command::run(&config.build.build, output.join(&down.file_name));
    bar_build.inc(1);

    bar_build.set_message(format!(
        "Packaging {}-{}",
        config.general.name, config.source.version
    ));
    command::run(&config.build.install, output.join(&down.file_name));
    bar_build.inc(1);

    bar_build.finish_with_message(format!(
        "Finished building {}-{}",
        config.general.name, config.source.version
    ));
}
