use std::path::Path;
use std::process::exit;

use log::error;
use tokio::pin;
use tokio_stream::Stream;
use tokio_util::{bytes::Buf, io::StreamReader};

use crate::fatal;

pub async fn decompress(
    stream: impl Stream<Item = Result<impl Buf, impl Into<std::io::Error>>>,
    output: &Path,
    file_name: &str,
) -> String {
    let mut file_name = String::from(file_name.trim());

    // TODO: create a more elegant and efficent way to determine compression algo and file name

    if file_name.ends_with(".tar.zst") {
        let decoder =
            async_compression::tokio::bufread::ZstdDecoder::new(StreamReader::new(stream));

        pin!(decoder);

        tokio_tar::Archive::new(decoder)
            .unpack(&output)
            .await
            .expect("Cannot unpack archive");

        file_name.truncate(file_name.len() - ".tar.zst".len())
    } else if file_name.ends_with(".tar.xz") {
        let decoder = async_compression::tokio::bufread::XzDecoder::new(StreamReader::new(stream));

        pin!(decoder);

        tokio_tar::Archive::new(decoder)
            .unpack(&output)
            .await
            .expect("Cannot unpack archive");

        file_name.truncate(file_name.len() - ".tar.xz".len())
    } else if file_name.ends_with(".tar.gz") {
        let decoder =
            async_compression::tokio::bufread::GzipDecoder::new(StreamReader::new(stream));

        pin!(decoder);

        tokio_tar::Archive::new(decoder)
            .unpack(&output)
            .await
            .expect("Cannot unpack archive");

        file_name.truncate(file_name.len() - ".tar.gz".len())
    } else if file_name.ends_with(".tar.bz2") {
        let decoder = async_compression::tokio::bufread::BzDecoder::new(StreamReader::new(stream));

        pin!(decoder);

        tokio_tar::Archive::new(decoder)
            .unpack(&output)
            .await
            .expect("Cannot unpack archive");

        file_name.truncate(file_name.len() - ".tar.bz2".len())
    } else {
        fatal!("Extension of \"{}\" not supported!", file_name);
    }

    file_name
}
