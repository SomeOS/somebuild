use std::cmp::min;
use std::path::Path;

use indicatif::{MultiProgress, ProgressBar};
use tokio_stream::StreamExt;
use url::Url;

use crate::decompress::decompress;
use crate::log::*;
use crate::{bars::ProgressStyle, somebuild_config::Config};

pub struct Download<'a> {
    bar: ProgressBar,
    config: &'a Config,
    output: &'a Path,
    #[allow(dead_code)]
    pub full_file_name: String,
    #[allow(dead_code)]
    pub file_name: String,
    #[allow(dead_code)]
    pub extension: String,
}

impl<'a> Download<'a> {
    pub fn new(multibar: &MultiProgress, config: &'a Config, output: &'a Path) -> Self {
        let bar = multibar.add(ProgressBar::new(1));
        let name = Url::parse(&config.source.url)
            .unwrap()
            .path_segments()
            .unwrap()
            .last()
            .unwrap()
            .trim()
            .to_string();

        let (full_file_name, file_name, extension) = if name.ends_with(".tar.zst") {
            (
                name.clone(),
                name.split_at(name.len() - ".tar.zst".len()).0.to_string(),
                ".tar.zst".to_string(),
            )
        } else if name.ends_with(".tar.xz") {
            (
                name.clone(),
                name.split_at(name.len() - ".tar.xz".len()).0.to_string(),
                ".tar.xz".to_string(),
            )
        } else if name.ends_with(".tar.gz") {
            (
                name.clone(),
                name.split_at(name.len() - ".tar.gz".len()).0.to_string(),
                ".tar.gz".to_string(),
            )
        } else if name.ends_with(".tar.bz2") {
            (
                name.clone(),
                name.split_at(name.len() - ".tar.bz2".len()).0.to_string(),
                ".tar.bz2".to_string(),
            )
        } else {
            fatal!("Extension of \"{}\" not supported!", name);
        };

        Self {
            bar,
            config,
            output,
            full_file_name,
            file_name,
            extension,
        }
    }

    pub async fn download(&self) {
        self.bar.set_style(ProgressStyle::Download.value());
        self.bar.set_message(format!(
            "Starting {}-{}",
            self.config.general.name, self.config.source.version
        ));

        self.bar.tick();

        let response = reqwest::get(&self.config.source.url).await.unwrap();

        let total_size = response.content_length().unwrap_or(0);

        self.bar.set_length(total_size);
        self.bar.set_message(format!(
            "Downloading {}-{}",
            self.config.general.name, self.config.source.version
        ));

        let mut hasher = blake3::Hasher::new();
        let mut downloaded: u64 = 0;

        let stream = response.bytes_stream().map(|result| {
            result
                .inspect(|result| {
                    let new = min(downloaded + (result.len() as u64), total_size);
                    downloaded = new;
                    self.bar.set_position(new);
                })
                .inspect(|result| {
                    hasher.update(result);
                })
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
        });

        decompress(stream, self.output, &self.extension).await;

        info!("Extract Folder:\t{}", self.file_name);

        let hash = hasher.finalize();

        if hash.to_string() != self.config.source.hash {
            fatal!(
                "Hash error for \"{}\" specified \n\t \"{}\" found\n\t \"{}\"",
                self.config.source.url,
                self.config.source.hash,
                hash.to_string()
            );
        }
    }

    pub fn finish(&self) {
        self.bar.finish_with_message(format!(
            "Finished downloading {}-{}",
            self.config.general.name, self.config.source.version
        ));
    }
}
