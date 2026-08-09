use std::{
    io::{PipeReader, Read},
    ops,
    os::fd::AsFd,
};

use async_io::Async;

use crate::constants::READER_BUFFER_SIZE;

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
        let mut chunk = [0; READER_BUFFER_SIZE];

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

#[cfg(test)]
mod tests {
    use super::AsyncOsReader;
    use std::io::{pipe, Write};

    #[tokio::test]
    async fn reads_all_bytes_until_pipe_closes() {
        let (reader, mut writer) = pipe().expect("pipe should be created");
        let expected = b"runner output".to_vec();
        let to_write = expected.clone();

        let writer_task = tokio::task::spawn_blocking(move || {
            writer
                .write_all(&to_write)
                .expect("pipe write should succeed");
        });

        let mut reader = AsyncOsReader::from(reader);
        let mut output = Vec::new();
        let bytes_read = reader
            .read_to_end(&mut output)
            .await
            .expect("pipe read should succeed");

        writer_task.await.expect("writer task should finish");
        assert_eq!(bytes_read, expected.len());
        assert_eq!(output, expected);
    }
}
