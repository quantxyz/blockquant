use futures::stream::StreamExt;
use mongodb::bson::doc;
use mongodb::{bson::Document, options::FindOptions, Client, Collection};

#[derive(Debug, Clone)]
pub struct ClientMongo {
    url: String,
    db_name: String,
}

impl ClientMongo {
    pub fn new(url: Option<String>, db_name: Option<String>) -> Self {
        let default_url = "mongodb://localhost:27017/".to_string();
        let default_db_name = "stocksdb".to_string();

        Self {
            url: url.unwrap_or(default_url),
            db_name: db_name.unwrap_or(default_db_name),
        }
    }
    pub fn with_db_name(db_name: String) -> Self {
        Self::new(None, Some(db_name))
    }
    pub async fn records_query(
        &self,
        collection_name: &str,
        query: Option<Document>,
        limit: Option<i64>,
        sort_col: Option<&str>,
        small_first: Option<bool>,
    ) -> Result<Vec<Document>, mongodb::error::Error> {
        let client = Client::with_uri_str(&self.url).await?;
        let db = client.database(&self.db_name);
        let collection: Collection<Document> = db.collection(collection_name);
        let query = query.unwrap_or_default();
        let limit = limit.unwrap_or(0);
        let sort_col = sort_col.unwrap_or("_id");
        let small_first = small_first.unwrap_or(true);

        let sort_order = if small_first { 1 } else { -1 };

        let find_options = FindOptions::builder()
            .sort(doc! {sort_col: sort_order})
            .limit(limit)
            .build();

        let mut cursor = collection.find(query, find_options).await?;

        let mut records = Vec::new();
        while let Some(result) = cursor.next().await {
            match result {
                Ok(document) => records.push(document),
                Err(e) => {
                    log::error!("error:{}", e);
                }
            }
        }

        Ok(records)
    }
}
