mod bars;
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
use tokio::process::Command;
use tokio_stream::StreamExt;
use tokio_util::io::StreamReader;
use url::Url;

use crate::bars::{create_multibar, ProgressStyle};
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
    multibar.println("Running Package Build").unwrap();

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
                hasher.update(result);

                let new = min(downloaded + (result.len() as u64), total_size);
                downloaded = new;
                bar.set_position(new);
            })
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    });

    let decoder = async_compression::tokio::bufread::ZstdDecoder::new(StreamReader::new(stream));

    tokio_tar::Archive::new(decoder)
        .unpack(&output)
        .await
        .expect("Cannot unpack archive");

    let hash = hasher.finalize();

    if hash.to_string() != config.source.hash {
        error!(
            "Hash error for \"{}\" specified \n\t \"{}\" found\n\t \"{}\"",
            config.source.url,
            config.source.hash,
            hash.to_string()
        );
    }
    bar.finish_with_message(format!(
        "Finished downloading {}-{}",
        config.general.name, config.source.version
    ));

    let url = Url::parse(&config.source.url).unwrap();
    let file_name = url.path_segments().unwrap().last().unwrap();

    let mut file_name: Vec<&str> = file_name.split('.').collect();

    file_name.pop();
    file_name.pop();

    let file_name = file_name.join(".");

    info!("Extraction Folder: {}", file_name);

    let bar_build = multibar.add(ProgressBar::new(3));
    bar_build.set_style(ProgressStyle::Build.value());

    bar_build.tick();

    bar_build.set_message(format!(
        "Setup {}-{}",
        config.general.name, config.source.version
    ));

    let configure_cmd = config.build.setup.replace(
        "%configure",
        format!(
            "./configure --prefix=/usr --docdir=/usr/share/doc/{}-{}",
            config.general.name, config.source.version
        )
        .trim(),
    );

    let mut options = ScriptOptions::new();

    options.working_directory = Some(output.join(&file_name));

    let (code, out, error) = run_script::run_script!(configure_cmd, options).unwrap();

    if code != 0 {
        error!("Configure failed with code: {}", code);
        error!("Configure failed with error: {}", error);
        fatal!("Configure failed with output: {}", out);
    }
    bar_build.inc(1);

    bar_build.set_message(format!(
        "Building {}-{}",
        config.general.name, config.source.version
    ));
    let make = Command::new("make")
        .current_dir(&output.join(&file_name))
        .spawn()
        .expect("failed to start build")
        .wait()
        .await
        .expect("failed to run build");

    if !make.success() {
        error!("Build failed with error: {}", make.code().unwrap());
        fatal!("Build command: {}", configure_cmd);
    }
    bar_build.inc(1);

    bar_build.set_message(format!(
        "Packaging {}-{}",
        config.general.name, config.source.version
    ));
    let make_install = Command::new("make")
        .current_dir(&output.join(&file_name))
        .args([
            format!("DESTDIR={}", output.to_str().unwrap()).as_str(),
            "install",
        ])
        .spawn()
        .expect("failed to start install")
        .wait()
        .await
        .expect("failed to run install");

    if !make_install.success() {
        error!(
            "Install failed with error: {}",
            make_install.code().unwrap()
        );
        fatal!("Install command: {}", configure_cmd);
    }
    bar_build.inc(1);

    bar_build.finish_with_message(format!(
        "Finished building {}-{}",
        config.general.name, config.source.version
    ));
}
