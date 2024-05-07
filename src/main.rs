mod paths;
mod somebuild_config;
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use indicatif_log_bridge::LogWrapper;
use log::{error, info};
use std::cmp::min;
use std::path::Path;
use std::process::exit;
use tokio::fs::{self, File};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio_stream::StreamExt;
use tokio_util::io::StreamReader;

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
    let logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .format_timestamp(None)
            .format_target(false)
            .build();

    let multibar: MultiProgress = MultiProgress::new();
    LogWrapper::new(multibar.clone(), logger)
        .try_init()
        .unwrap();
    let sty = ProgressStyle::with_template(
        "{spinner:.green} [{elapsed}] {wide_bar:.cyan/blue} {bytes}/{total_bytes} {bytes_per_sec} {msg} ({eta})",
    )
    .unwrap()
    .progress_chars("#>-");

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
    bar.set_style(sty.clone());
    bar.set_message(format!(
        "Starting {}-{}",
        config.general.name, config.source.version
    ));

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

    let mut archive = tokio_tar::Archive::new(decoder);

    archive
        .unpack(&output)
        .await
        .expect("Cannot unpack archive");

    bar.finish_with_message(format!(
        "Finished downloading {}-{}",
        config.general.name, config.source.version
    ));

    multibar.remove(&bar);

    let hash = hasher.finalize();

    if hash.to_string() != config.source.hash {
        error!(
            "Hash error for \"{}\" specified \n\t \"{}\" found\n\t \"{}\"",
            config.source.url,
            config.source.hash,
            hash.to_string()
        );
    }

    info!("Running Configure!");
    let cmd_output = Command::new("./configure")
        .current_dir(&output.join("xz-5.4.6"))
        .args([
            "--prefix=/usr",
            "--docdir=/usr/share/doc/xz-5.4.6",
            "--disable-static",
        ])
        .output()
        .await
        .expect("failed to run configure");
    io::stdout().write_all(&cmd_output.stdout).await.unwrap();
    io::stderr().write_all(&cmd_output.stderr).await.unwrap();

    info!("Running make!");
    let cmd_output = Command::new("make")
        .current_dir(&output.join("xz-5.4.6"))
        .output()
        .await
        .expect("failed to run make");
    io::stdout().write_all(&cmd_output.stdout).await.unwrap();
    io::stderr().write_all(&cmd_output.stderr).await.unwrap();

    info!("Running make install!");
    let cmd_output = Command::new("make")
        .current_dir(&output.join("xz-5.4.6"))
        .args([
            "DESTDIR=".to_owned() + output.to_str().unwrap(),
            "install".to_string(),
        ])
        .output()
        .await
        .expect("failed to run make");
    io::stdout().write_all(&cmd_output.stdout).await.unwrap();
    io::stderr().write_all(&cmd_output.stderr).await.unwrap();
}
