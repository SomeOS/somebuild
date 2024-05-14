mod bars;
mod decompress;
mod paths;
mod somebuild_config;

use clap::Parser;
use indicatif::ProgressBar;
use log::{error, info};
use run_script::ScriptOptions;
use std::cmp::min;
use std::path::Path;
use std::process::exit;
use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;
use tokio_stream::StreamExt;
use url::Url;

use crate::bars::{create_multibar, ProgressStyle};
use crate::decompress::decompress;
use crate::paths::normalize_path;
use crate::somebuild_config::Config;

#[macro_export]
macro_rules! fatal {
    ( $($var:tt)* ) => {
        error!($($var)*);
        exit(1);
    };
}

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

    let bar = multibar.add(ProgressBar::new(1));
    bar.set_style(ProgressStyle::Download.value());
    bar.set_message(format!(
        "Starting {}-{}",
        config.general.name, config.source.version
    ));

    bar.tick();

    let response = reqwest::get(&config.source.url).await.unwrap();

    let total_size = response.content_length().unwrap_or(0);

    bar.set_length(total_size);
    bar.set_message(format!(
        "Downloading {}-{}",
        config.general.name, config.source.version
    ));

    let mut hasher = blake3::Hasher::new();
    let mut downloaded: u64 = 0;

    let stream = response.bytes_stream().map(|result| {
        result
            .inspect(|result| {
                let new = min(downloaded + (result.len() as u64), total_size);
                downloaded = new;
                bar.set_position(new);
            })
            .inspect(|result| {
                hasher.update(result);
            })
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    });

    let file_name = decompress(
        stream,
        &output,
        Url::parse(&config.source.url)
            .unwrap()
            .path_segments()
            .unwrap()
            .last()
            .unwrap(),
    )
    .await;
    let file_name = file_name.as_str();

    info!("Extract Folder:\t{}", file_name);

    let hash = hasher.finalize();

    if hash.to_string() != config.source.hash {
        fatal!(
            "Hash error for \"{}\" specified \n\t \"{}\" found\n\t \"{}\"",
            config.source.url,
            config.source.hash,
            hash.to_string()
        );
    }

    let bar_build = multibar.add(ProgressBar::new(3));
    bar_build.set_style(ProgressStyle::Build.value());

    bar_build.set_message(format!(
        "Setup {}-{}",
        config.general.name, config.source.version
    ));

    bar.finish_with_message(format!(
        "Finished downloading {}-{}",
        config.general.name, config.source.version
    ));

    bar_build.tick();

    let configure_cmd = config
        .build
        .setup
        .replace("%configure", "./configure --prefix=/usr")
        .trim()
        .to_string();

    let mut options = ScriptOptions::new();
    options.working_directory = Some(output.join(file_name));

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
    options.working_directory = Some(output.join(file_name));

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
    options.working_directory = Some(output.join(file_name));

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
