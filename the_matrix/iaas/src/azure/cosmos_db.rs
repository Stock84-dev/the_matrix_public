use std::borrow::Cow;
use std::sync::Arc;

use azure_core::HttpClient;
use azure_cosmos::prelude::{AuthorizationToken, CosmosClient, DatabaseClient};
use azure_cosmos::responses::GetDocumentResponse;
use azure_cosmos::CosmosEntity;
use config::CONFIG;
use mouse::error::{BoxErrorExt, Result};
use serde::{Deserialize, Serialize};

/// NOTE: new models should have their ids bigger or equal than max model id
pub async fn update_model_document<'a>(model_doc: &ModelDocument<'a>) -> Result<()> {
    let local_max = model_doc.models.iter().map(|x| x.id).max().unwrap();
    maybe_set_max_id(&"models".into(), local_max).await?;
    DATABASE_CLIENT
        .clone()
        .into_collection_client("models")
        .into_document_client(model_doc.name.as_ref(), model_doc.name.as_ref())?
        .replace_document()
        .execute(model_doc)
        .await
        .sized()?;
    Ok(())
}

pub async fn get_exchange_id<'a>(exchange_name: &'a String) -> Result<u16> {
    let document_client = DATABASE_CLIENT
        .clone()
        .into_collection_client("exchanges")
        .into_document_client(exchange_name, exchange_name)?;
    return match document_client
        .get_document()
        .execute::<ExchangeDocument>()
        .await
        .sized()?
    {
        GetDocumentResponse::Found(document) => Ok(document.document.document.id),
        GetDocumentResponse::NotFound(_) => {
            let id = increase_max_id(&"exchanges".into()).await?;
            let doc = ExchangeDocument {
                name: Cow::Borrowed(exchange_name),
                id: id as u16,
            };
            document_client
                .collection_client()
                .create_document()
                .execute(&doc)
                .await
                .sized()?;
            Ok(doc.id)
        }
    };
}

pub async fn get_model_max_id() -> Result<u32> {
    let table_name = String::from("models");
    let client = DATABASE_CLIENT
        .clone()
        .into_collection_client("ids")
        .into_document_client(&table_name, &table_name)?;
    match client
        .get_document()
        .execute::<MaxIdDocument>()
        .await
        .sized()?
    {
        GetDocumentResponse::Found(doc) => Ok(doc.document.document.max_id),
        GetDocumentResponse::NotFound(_) => Ok(0),
    }
}

pub async fn get_model_document<'a>(model_name: &'a String) -> Result<ModelDocument<'a>> {
    let document_client = DATABASE_CLIENT
        .clone()
        .into_collection_client("models")
        .into_document_client(model_name, model_name)?;
    return match document_client
        .get_document()
        .execute::<ModelDocument>()
        .await
        .sized()?
    {
        GetDocumentResponse::Found(document) => Ok(document.document.document),
        GetDocumentResponse::NotFound(_) => {
            // Creating document since it doesn't exist
            maybe_set_max_id(&"models".into(), 0).await?;
            let doc = ModelDocument {
                name: Cow::Borrowed(model_name),
                models: vec![],
            };
            document_client
                .collection_client()
                .create_document()
                .execute(&doc)
                .await
                .sized()?;
            Ok(doc)
        }
    };
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelDocument<'a> {
    // Id gets generated if it doesn't exist
    #[serde(rename = "id")]
    pub name: Cow<'a, String>,
    #[serde(rename = "m")]
    pub models: Vec<ModelSubDocument>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelSubDocument {
    #[serde(rename = "i")]
    pub id: u32,
    #[serde(rename = "v")]
    pub values: Vec<f32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ExchangeDocument<'a> {
    #[serde(rename = "id")]
    name: Cow<'a, String>,
    #[serde(rename = "i")]
    id: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct MaxIdDocument<'a> {
    #[serde(rename = "id")]
    name: Cow<'a, String>,
    #[serde(rename = "c")]
    max_id: u32,
}

impl<'a> CosmosEntity<'a> for ModelDocument<'a> {
    type Entity = &'a String;

    fn partition_key(&'a self) -> Self::Entity {
        &self.name
    }
}

impl<'a> CosmosEntity<'a> for ExchangeDocument<'a> {
    type Entity = &'a String;

    fn partition_key(&'a self) -> Self::Entity {
        &self.name
    }
}

impl<'a> CosmosEntity<'a> for MaxIdDocument<'a> {
    type Entity = &'a String;

    fn partition_key(&'a self) -> Self::Entity {
        &self.name
    }
}

lazy_static::lazy_static! {
    static ref DATABASE_CLIENT: DatabaseClient = {
        let iaas = CONFIG.iaas.as_ref().unwrap();
        let authorization_token = AuthorizationToken::primary_from_base64(&iaas.cosmos_master_key).unwrap();
        let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(reqwest::Client::new()));
        let client = CosmosClient::new(http_client, iaas.cosmos_account.clone(), authorization_token);
        client.into_database_client(&iaas.cosmos_core_db_name)
    };
}

/// Increases max id on db side and return it.
/// Note: there are no transactions so there could be ids that don't have their items
async fn increase_max_id(doc_name: &String) -> Result<u32> {
    let client = DATABASE_CLIENT
        .clone()
        .into_collection_client("ids")
        .into_document_client(doc_name, doc_name)?;
    match client
        .get_document()
        .execute::<MaxIdDocument>()
        .await
        .sized()?
    {
        GetDocumentResponse::Found(doc) => {
            let mut doc = doc.document.document;
            doc.max_id += 1;
            client.replace_document().execute(&doc).await.sized()?;
            Ok(doc.max_id)
        }
        GetDocumentResponse::NotFound(_) => {
            let doc = MaxIdDocument {
                name: Cow::Borrowed(doc_name),
                max_id: 0,
            };
            client
                .collection_client()
                .create_document()
                .execute(&doc)
                .await
                .sized()?;
            Ok(0)
        }
    }
}

/// Sets max id if `local_max` is bigger
async fn maybe_set_max_id(doc_name: &String, local_max: u32) -> Result<u32> {
    let client = DATABASE_CLIENT
        .clone()
        .into_collection_client("ids")
        .into_document_client(doc_name, doc_name)?;
    match client
        .get_document()
        .execute::<MaxIdDocument>()
        .await
        .sized()?
    {
        GetDocumentResponse::Found(doc) => {
            let mut doc = doc.document.document;
            if local_max >= doc.max_id {
                doc.max_id = local_max + 1;
                client.replace_document().execute(&doc).await.sized()?;
            }
            Ok(doc.max_id)
        }
        GetDocumentResponse::NotFound(_) => {
            let doc = MaxIdDocument {
                name: Cow::Borrowed(doc_name),
                max_id: local_max,
            };
            client
                .collection_client()
                .create_document()
                .execute(&doc)
                .await
                .sized()?;
            Ok(local_max)
        }
    }
}
