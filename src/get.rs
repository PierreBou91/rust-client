use reqwest::{header, Client};

use crate::Env;

pub async fn get_study_status(
    env: Env,
    key: String,
    study_id: String,
) -> Result<reqwest::Response, reqwest::Error> {
    let milvue_api_url = match env {
        Env::Dev => "redacted/v3/studies",
        Env::Staging => "redacted/v3/studies",
        Env::Prod => "redacted/v3/studies",
    };

    let mut headers = header::HeaderMap::new();

    let mut api_header = header::HeaderValue::from_str(&key).unwrap();
    api_header.set_sensitive(true);
    headers.insert("x-goog-meta-owner", api_header);
    headers.insert(
        "Accept",
        header::HeaderValue::from_str("application/json").unwrap(),
    );

    let client = Client::builder()
        .default_headers(headers)
        // .https_only(true)
        .build()
        .unwrap();

    let response = client
        .get(format!("{}/{}/status", milvue_api_url, study_id))
        .send()
        .await
        .unwrap();

    Ok(response)
}
