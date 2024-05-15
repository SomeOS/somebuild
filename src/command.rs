use std::path::PathBuf;

use run_script::ScriptOptions;

use crate::log::*;

pub fn run(command: &str, path: PathBuf) {
    let configure_cmd = command
        .replace("%configure", "./configure --prefix=/usr")
        .replace(
            "%make_install",
            format!("make DESTDIR={} install", path.join("..").to_str().unwrap()).trim(),
        )
        .replace("%make", "make")
        .trim()
        .to_string();

    let mut options = ScriptOptions::new();
    options.working_directory = Some(path);

    let (code, out, error) = run_script::run_script!(configure_cmd, options).unwrap();
    if code != 0 {
        error!("Setup failed with command: {}", configure_cmd);
        error!("Setup failed with code: {}", code);
        error!("Setup failed with error: {}", error);
        fatal!("Setup failed with output: {}", out);
    }
}
