mod bars;
mod decompress;
mod downloader;
mod log;
mod paths;
mod somebuild_config;

use clap::Parser;
use indicatif::ProgressBar;

use run_script::ScriptOptions;
use std::path::Path;
use std::process::exit;
use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;

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

    bar_build.set_message(format!(
        "Setup {}-{}",
        config.general.name, config.source.version
    ));

    bar_build.tick();

    down.finish();

    let configure_cmd = config
        .build
        .setup
        .replace("%configure", "./configure --prefix=/usr")
        .trim()
        .to_string();

    let mut options = ScriptOptions::new();
    options.working_directory = Some(output.join(&down.file_name));

    let (code, out, error) = run_script::run_script!(configure_cmd, options).unwrap();
    if code != 0 {
        error!("Setup failed with command: {}", configure_cmd);
        error!("Setup failed with code: {}", code);
        error!("Setup failed with error: {}", error);
        fatal!("Setup failed with output: {}", out);
    }
    bar_build.inc(1);

    bar_build.set_message(format!(
        "Building {}-{}",
        config.general.name, config.source.version
    ));

    let make_cmd = config
        .build
        .build
        .replace("%make", "make")
        .trim()
        .to_string();

    let mut options = ScriptOptions::new();
    options.working_directory = Some(output.join(&down.file_name));

    let (code, out, error) = run_script::run_script!(make_cmd, options).unwrap();
    if code != 0 {
        error!("Build failed with command: {}", make_cmd);
        error!("Build failed with code: {}", code);
        error!("Build failed with error: {}", error);
        fatal!("Build failed with output: {}", out);
    }
    bar_build.inc(1);

    bar_build.set_message(format!(
        "Packaging {}-{}",
        config.general.name, config.source.version
    ));

    let make_install_cmd = config
        .build
        .install
        .replace(
            "%make_install",
            format!("make DESTDIR={} install", output.to_str().unwrap()).trim(),
        )
        .trim()
        .to_string();

    let mut options = ScriptOptions::new();
    options.working_directory = Some(output.join(&down.file_name));

    let (code, out, error) = run_script::run_script!(make_install_cmd, options).unwrap();
    if code != 0 {
        error!("Packaging failed with command: {}", make_install_cmd);
        error!("Packaging failed with code: {}", code);
        error!("Packaging failed with error: {}", error);
        fatal!("Packaging failed with output: {}", out);
    }

    bar_build.finish_with_message(format!(
        "Finished building {}-{}",
        config.general.name, config.source.version
    ));
}
