use std::path::Path;

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::somebuild_config::Config;

pub async fn parse(input: &Path) -> Config {
    let mut somebuild_file = File::open(input.join("SOMEBUILD.toml"))
        .await
        .expect("Failed to open SOMEBUILD.toml!");
    let mut config_str = String::new();

    somebuild_file
        .read_to_string(&mut config_str)
        .await
        .expect("Failed to read SOMEBUILD.toml!");

    toml::from_str(&config_str).expect("Failed to parse SOMEBUILD.toml!")
}
