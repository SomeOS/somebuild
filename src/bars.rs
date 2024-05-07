use indicatif::MultiProgress;
use indicatif_log_bridge::LogWrapper;

pub enum ProgressStyle {
    Download,
    Build,
}

impl ProgressStyle {
    pub fn value(&self) -> indicatif::ProgressStyle {
        match *self {
            Self::Download => indicatif::ProgressStyle::with_template(
                "{spinner:.green} [{elapsed}] {wide_bar:.cyan/blue} {bytes}/{total_bytes} {bytes_per_sec} {msg} ({eta})",
            ).unwrap(),
            Self::Build => indicatif::ProgressStyle::with_template(
                "{spinner:.green} [{elapsed}] {wide_bar:.cyan/blue} {pos}/{len} {msg}",
            ).unwrap(),
        }.progress_chars("#>-")
    }
}

pub fn create_multibar() -> MultiProgress {
    let logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .format_timestamp(None)
            .format_target(false)
            .build();

    let multibar = MultiProgress::new();

    LogWrapper::new(multibar.clone(), logger)
        .try_init()
        .unwrap();

    multibar
}
