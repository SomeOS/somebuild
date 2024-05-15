use std::path::Path;
use tokio::pin;
use tokio_stream::Stream;
use tokio_util::{bytes::Buf, io::StreamReader};

use crate::log::*;

pub async fn decompress(
    stream: impl Stream<Item = Result<impl Buf, impl Into<std::io::Error>>>,
    output: &Path,
    extension: &str,
) {
    // TODO: create a more elegant and efficent way to determine compression algo and file name

    match extension {
        ".tar.zst" => {
            let decoder =
                async_compression::tokio::bufread::ZstdDecoder::new(StreamReader::new(stream));

            pin!(decoder);

            tokio_tar::Archive::new(decoder)
                .unpack(&output)
                .await
                .expect("Cannot unpack archive");
        }
        ".tar.xz" => {
            let decoder =
                async_compression::tokio::bufread::XzDecoder::new(StreamReader::new(stream));

            pin!(decoder);

            tokio_tar::Archive::new(decoder)
                .unpack(&output)
                .await
                .expect("Cannot unpack archive");
        }
        ".tar.gz" => {
            let decoder =
                async_compression::tokio::bufread::GzipDecoder::new(StreamReader::new(stream));

            pin!(decoder);

            tokio_tar::Archive::new(decoder)
                .unpack(&output)
                .await
                .expect("Cannot unpack archive");
        }
        ".tar.bz2" => {
            let decoder =
                async_compression::tokio::bufread::BzDecoder::new(StreamReader::new(stream));

            pin!(decoder);

            tokio_tar::Archive::new(decoder)
                .unpack(&output)
                .await
                .expect("Cannot unpack archive");
        }
        _ => fatal!("Extension of \"{}\" not supported!", extension),
    }
}
