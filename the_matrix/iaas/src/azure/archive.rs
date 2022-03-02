use std::fmt::Write;
use std::sync::Arc;

use azure_core::errors::AzureError;
use azure_core::HttpClient;
use azure_storage::blob::prelude::{
    AsBlobClient, AsContainerClient, BlobBlockType, BlobClient, BlockList,
};
use azure_storage::core::prelude::{AsStorageClient, StorageAccountClient};
use azure_storage::{AccessTier, BlockId, Hash};
use bytes::Bytes;
use config::CONFIG;
use mouse::error::{BoxErrorExt, Result};
use speedy::Writable;
use tokio::io::{AsyncRead, AsyncReadExt};

pub async fn archive(
    blob_name: &str,
    reader: &mut (impl AsyncRead + Unpin),
    size_hint: usize,
) -> Result<()> {
    let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(reqwest::Client::new()));
    let iaas = CONFIG.iaas.as_ref().unwrap();

    let storage_account_client = StorageAccountClient::new_access_key(
        http_client.clone(),
        &iaas.storage_account,
        &iaas.storage_key,
    );
    let storage_client = storage_account_client.as_storage_client();
    let blob = storage_client
        .as_container_client("archive")
        .as_blob_client(blob_name);
    blob.put_block_blob(Bytes::new()).execute().await.sized()?;
    let mut block_list = BlockList::default();
    // max block size for preview version is 4000 MiB
    const MAX_BLOCK_SIZE: usize = 100 * (1 << 20);

    let mut buf = vec![
        0u8;
        if size_hint < MAX_BLOCK_SIZE {
            size_hint
        } else {
            MAX_BLOCK_SIZE
        }
    ];
    let mut pos = 0;
    loop {
        let read = reader.read(&mut buf[pos..]).await?;
        pos += read;
        if pos == buf.len() {
            let block_id = Writable::write_to_vec(&(block_list.blocks.len() as u16))?.into();
            upload(&blob, &buf, &block_id).await?;
            block_list.blocks.push(BlobBlockType::Uncommitted(block_id));
            pos = 0;
        }
        if read == 0 {
            let block_id = Writable::write_to_vec(&(block_list.blocks.len() as u16))?.into();
            upload(&blob, &buf[..pos], &block_id).await?;
            block_list.blocks.push(BlobBlockType::Uncommitted(block_id));
            break;
        }
    }
    blob.put_block_list(&block_list)
        .access_tier(AccessTier::Archive)
        .execute()
        .await
        .sized()?;
    Ok(())
}

async fn upload(client: &BlobClient, data: &[u8], block_id: &BlockId) -> Result<()> {
    let hash = Hash::MD5(md5::compute(&data).into());
    loop {
        if let Err(e) = client
            .put_block(block_id.clone(), data.to_owned())
            .hash(&hash)
            .execute()
            .await
        {
            if let Some(ae) = e.downcast_ref::<AzureError>() {
                match ae {
                    AzureError::UnexpectedHTTPResult(result) => {
                        let mut message = String::new();
                        write!(&mut message, "{:#?}", result)?;
                        if !message.contains("The MD5 value specified in the request did not match with the MD5 value calculated by the server.") {
                            throw!("{:#?}", e);
                        }
                        continue;
                    }
                    _ => throw!("{:#?}", e),
                }
            } else {
                throw!("{:#?}", e);
            }
        }
        break;
    }
    Ok(())
}
