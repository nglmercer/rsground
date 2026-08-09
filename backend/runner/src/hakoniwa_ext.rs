use std::{
    io::{PipeReader, Read},
    ops,
    os::fd::AsFd,
};

use async_io::Async;

pub struct AsyncOsReader(Async<PipeReader>);

impl From<PipeReader> for AsyncOsReader {
    fn from(value: PipeReader) -> Self {
        Self(Async::new(value).expect("Cannot create async wrapper"))
    }
}

impl AsFd for AsyncOsReader {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsyncOsReader {
    pub async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read_with(|mut reader| reader.read(buf)).await
    }

    pub async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let mut total_bytes = 0;
        let mut chunk = [0; 8192];

        loop {
            let read_bytes = self.read(&mut chunk).await?;

            if read_bytes == 0 {
                break;
            }

            buf.extend_from_slice(&chunk[..read_bytes]);
            total_bytes += read_bytes;
        }

        Ok(total_bytes)
    }
}

impl ops::Deref for AsyncOsReader {
    type Target = PipeReader;

    fn deref(&self) -> &Self::Target {
        self.0.get_ref()
    }
}
