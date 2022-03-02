use std::io::{ErrorKind, SeekFrom};

use mouse::ext::{VecBytemuckExt};
use mouse::num::traits::ToPrimitive;
use mouse::traits::AsyncReadSeek;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::structs::BatchHeader;

pub struct OutputReader<R> {
    n_output_params: usize,
    buf: Vec<u8>,
    reader: R,
}

impl<R: AsyncReadSeek + Unpin> OutputReader<R> {
    pub fn new(decoder: R, n_output_params: usize) -> Self {
        debug!("{:#?}", n_output_params);
        let size = bincode::serialized_size(&BatchHeader {
            start_combination: 0,
            end_combination_inclusive: 0,
            block_size: 0,
            n_combinations: 0,
            ranges: vec![Default::default(); n_output_params],
        })
        .unwrap();
        //        let header_size = (BatchHeader::size() - std::mem::size_of::<Vec<()>>() + 8);
        let buf = vec![0; size as usize];
        //        let buf = vec![0; n_output_params * 8 + header_size];
        println!("{} {}", size, buf.len());
        Self {
            n_output_params,
            buf,
            reader: decoder,
        }
    }

    pub async fn seek_start(&mut self) -> std::io::Result<u64> {
        self.reader.seek(SeekFrom::Start(0)).await
    }

    pub async fn read_header(&mut self) -> Result<BatchHeader, OutputReadError> {
        self.reader
            .read_exact(&mut self.buf)
            .await
            .map_err(|e| match e.kind() {
                ErrorKind::UnexpectedEof => OutputReadError::AllRead,
                _ => OutputReadError::Other(e.into()),
            })?;
        Ok(
            bincode::deserialize(&self.buf).map_err(|e| OutputReadError::Other(e.into()))
//            .map_err(|_| OutputReadError::CorruptedSource)
            ?,
        )
    }

    pub async fn skip(&mut self, header: &BatchHeader) -> Result<u64, OutputReadError> {
        let size = header
            .block_size
            .to_i64()
            .ok_or(OutputReadError::CorruptedSource)?;
        Ok(self.reader.seek(SeekFrom::Current(size)).await?)
    }

    pub async fn read_block(
        &mut self,
        header: &BatchHeader,
        data: &mut Vec<u8>,
    ) -> std::io::Result<usize> {
        let block_size = header.block_size as usize;
        data.alloc_set_len(block_size);
        self.reader.read_exact(data).await
    }
}

#[derive(Error, Debug)]
#[error("{0:#?}")]
pub enum OutputReadError {
    #[error("Data source is corrupted")]
    CorruptedSource,
    #[error("There is no more thata")]
    AllRead,
    Other(#[from] mouse::error::Error),
}

impl From<std::io::Error> for OutputReadError {
    fn from(e: std::io::Error) -> Self {
        OutputReadError::Other(e.into())
    }
}
