use std::path::Path;
use std::process::exit;

use log::error;
use tokio::pin;
use tokio_stream::Stream;
use tokio_util::{bytes::Buf, io::StreamReader};

use crate::fatal;

pub async fn decompress<'a>(
    stream: impl Stream<Item = Result<impl Buf, impl Into<std::io::Error>>>,
    output: &Path,
    file_name: &'a str,
) -> &'a str {
    if file_name.ends_with(".tar.zst") {
        let decoder =
            async_compression::tokio::bufread::ZstdDecoder::new(StreamReader::new(stream));

        pin!(decoder);

        tokio_tar::Archive::new(decoder)
            .unpack(&output)
            .await
            .expect("Cannot unpack archive");

        &file_name[..".tar.zst".len()]
    } else {
        fatal!("Extension of \"{}\" not supported!", file_name);
    }
}
