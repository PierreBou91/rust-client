use dicom_object::{file::ReadPreamble, FileDicomObject, InMemDicomObject, OpenFileOptions};
use multer::Multipart;
use reqwest::{header, Client};
use std::io::Cursor;

use crate::{MilvueParams, MilvueUrl, StatusResponse};

/// Fetches DICOM files from a study in the default environment.
///
/// # Arguments
///
/// * `key` - A string slice that holds the API key
/// * `study_id` - A string slice that holds the ID of the study
/// * `milvue_params` - A reference to MilvueParams containing parameters for the request
///
/// # Returns
///
/// * A Result containing a vector of DICOM files or an error
pub async fn get(
    key: &str,
    study_id: &str,
    milvue_params: &MilvueParams,
) -> Result<Vec<FileDicomObject<InMemDicomObject>>, Box<dyn std::error::Error>> {
    get_with_url(&MilvueUrl::default(), key, study_id, milvue_params).await
}

/// Fetches DICOM files from a study in the specified environment.
///
/// # Arguments
///
/// * `env` - A reference to MilvueUrl that specifies the environment
/// * `key` - A string slice that holds the API key
/// * `study_id` - A string slice that holds the ID of the study
/// * `milvue_params` - A reference to MilvueParams containing parameters for the request
///
/// # Returns
///
/// * A Result containing a vector of DICOM files or an error
pub async fn get_with_url(
    env: &MilvueUrl,
    key: &str,
    study_id: &str,
    milvue_params: &MilvueParams,
) -> Result<Vec<FileDicomObject<InMemDicomObject>>, Box<dyn std::error::Error>> {
    let milvue_api_url = format!("{}/v3/studies/{}", MilvueUrl::get_url(env)?, study_id);

    let mut headers = header::HeaderMap::new();

    let mut api_header = header::HeaderValue::from_str(key).unwrap();
    api_header.set_sensitive(true);
    headers.insert("x-goog-meta-owner", api_header);
    headers.insert(
        "Accept",
        header::HeaderValue::from_str("application/json").unwrap(),
    );

    let client = Client::builder().default_headers(headers).build().unwrap();

    let response = client
        .get(milvue_api_url)
        .query(milvue_params.to_query_param().as_slice())
        .send()
        .await?;

    println!("Response {:#?}", response.headers());

    let boundary = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|header_value| header_value.to_str().ok())
        .and_then(|content_type| {
            let parts: Vec<_> = content_type.split("boundary=").collect();
            if parts.len() == 2 {
                Some(parts[1].to_string())
            } else {
                None
            }
        })
        .unwrap(); // add support for the none case

    println!("Boundary: {}", boundary);
    let body = response.bytes().await?;
    let cursor = Cursor::new(body);
    let mut multipart = Multipart::with_reader(cursor, boundary);
    println!("Multipart: {:#?}", multipart);
    let mut dicoms = Vec::new();

    while let Some(field) = multipart.next_field().await? {
        println!("Field: {:#?}", field);
        let file_bytes = field.bytes().await?;
        println!("File bytes length: {:#?}", file_bytes.len());
        // println!("File bytes: {:#?}", file_bytes);

        // Print the 10 bytes after an offset of 128 bytes for each field
        if file_bytes.len() > 128 {
            let offset = 128;
            let end = std::cmp::min(file_bytes.len(), offset + 8);
            let slice = &file_bytes[offset..end];
            // println!("10 bytes after an offset of 128: {:?}", slice);
            println!(
                "10 bytes after an offset of 128: {}",
                String::from_utf8_lossy(slice)
            );
        } else {
            println!("File bytes length is less than or equal to 128.");
        }
        match OpenFileOptions::new()
            .read_preamble(ReadPreamble::Always)
            .from_reader(Cursor::new(file_bytes.clone()))
        {
            Ok(dicom_file) => {
                dicoms.push(dicom_file);
            }
            Err(err) => {
                eprintln!("Error reading DICOM file: {:?}", err);
            }
        }

        // dicoms.push(dicom_file);
    }

    Ok(dicoms)
}

/// Fetches the status of a study in the default environment.
///
/// # Arguments
///
/// * `key` - A string slice that holds the API key
/// * `study_id` - A string slice that holds the ID of the study
///
/// # Returns
///
/// * A Result containing the response from the server or an error
pub async fn get_study_status(
    key: &str,
    study_id: &str,
) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
    get_study_status_with_url(&MilvueUrl::default(), key, study_id).await
}

/// Fetches the status of a study in the specified environment.
///
/// # Arguments
///
/// * `env` - A reference to MilvueUrl that specifies the environment
/// * `key` - A string slice that holds the API key
/// * `study_id` - A string slice that holds the ID of the study
///
/// # Returns
///
/// * A Result containing the response from the server or an error
pub async fn get_study_status_with_url(
    env: &MilvueUrl,
    key: &str,
    study_id: &str,
) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
    let milvue_api_url = format!("{}/v3/studies/{}", MilvueUrl::get_url(env)?, study_id);

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
        .get(format!("{}/status", milvue_api_url))
        .send()
        .await
        .unwrap();

    Ok(response)
}

/// Waits for a study to be done in the default environment.
///
/// # Arguments
///
/// * `key` - A string slice that holds the API key
/// * `study_id` - A string slice that holds the ID of the study
///
/// # Returns
///
/// * A Result indicating success (empty Ok value) or an error
pub async fn wait_for_done(key: &str, study_id: &str) -> Result<(), reqwest::Error> {
    wait_for_done_with_url(&MilvueUrl::default(), key, study_id).await
}

/// Waits for a study to be done in the specified environment.
///
/// # Arguments
///
/// * `env` - A reference to MilvueUrl that specifies the environment
/// * `key` - A string slice that holds the API key
/// * `study_id` - A string slice that holds the ID of the study
///
/// # Returns
///
/// * A Result indicating success (empty Ok value) or an error
pub async fn wait_for_done_with_url(
    env: &MilvueUrl,
    key: &str,
    study_id: &str,
) -> Result<(), reqwest::Error> {
    let mut status_response = get_study_status_with_url(env, key, study_id).await.unwrap();

    let mut status_body: StatusResponse = status_response.json().await.unwrap();

    while status_body.status != "done" {
        println!("Status: {}", status_body.status);
        status_response = get_study_status_with_url(env, key, study_id).await.unwrap();
        status_body = status_response.json().await.unwrap();
        println!("Waiting for done...");
        std::thread::sleep(std::time::Duration::from_secs(3));
    }

    Ok(())
}
