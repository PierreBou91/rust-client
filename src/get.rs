use reqwest::{header, Client};

use crate::{Env, MilvueParams, StatusResponse};

pub async fn get(
    env: &Env,
    key: &str,
    study_id: &str,
    milvue_params: &MilvueParams,
) -> Result<reqwest::Response, reqwest::Error> {
    let milvue_api_url = match env {
        Env::Dev => "redacted/v3/studies",
        Env::Staging => "redacted/v3/studies",
        Env::Prod => "redacted/v3/studies",
    };

    let mut headers = header::HeaderMap::new();

    let mut api_header = header::HeaderValue::from_str(key).unwrap();
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
        .get(format!("{}/{}", milvue_api_url, study_id))
        .query(milvue_params.to_query_param().as_slice())
        .send()
        .await
        .unwrap();

    Ok(response)
}

pub async fn get_study_status(
    env: &Env,
    key: &str,
    study_id: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    let milvue_api_url = match env {
        Env::Dev => "redacted/v3/studies",
        Env::Staging => "redacted/v3/studies",
        Env::Prod => "redacted/v3/studies",
    };

    let mut headers = header::HeaderMap::new();

    let mut api_header = header::HeaderValue::from_str(key).unwrap();
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

pub async fn wait_for_done(env: &Env, key: &str, study_id: &str) -> Result<(), reqwest::Error> {
    let mut status_response = get_study_status(env, key, study_id).await.unwrap();

    let mut status_body: StatusResponse = status_response.json().await.unwrap();
    print!("Status: {}", status_body.status);

    while status_body.status != "done" {
        status_response = get_study_status(env, key, study_id).await.unwrap();
        status_body = status_response.json().await.unwrap();
        println!("Waiting for done...");
        std::thread::sleep(std::time::Duration::from_secs(3));
    }

    Ok(())
}
