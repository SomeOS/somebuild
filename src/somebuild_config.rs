use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub general: ConfigGeneral,
    pub source: ConfigSource,
    pub build: ConfigBuild,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigGeneral {
    pub name: String,
    pub description: String,
    pub homepage: String,
    pub licences: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigSource {
    pub version: String,
    pub url: String,
    pub hash: String,
    pub release: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigBuild {
    pub setup: String,
    pub build: String,
    pub options: ConfigBuildOptions,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigBuildOptions {
    #[serde(default = "_default_clang")]
    pub compiler: String,
    #[serde(default = "_default_true")]
    pub with_lto: bool,
}

fn _default_clang() -> String {
    "clang".to_string()
}

const fn _default_true() -> bool {
    true
}

const fn _default_false() -> bool {
    false
}
