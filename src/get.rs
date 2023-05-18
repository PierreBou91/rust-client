use dicom_object::{file::ReadPreamble, FileDicomObject, InMemDicomObject, OpenFileOptions};
use multer::Multipart;
use reqwest::{header, Client};
use std::io::Cursor;

use crate::{structs::MilvueError, MilvueParams, MilvueUrl, StatusResponse};

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
) -> Result<Option<Vec<FileDicomObject<InMemDicomObject>>>, MilvueError> {
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
/// * An Option containing a vector of DICOM files or None, in the case of a None, this means that there is no output
/// for the given configuration. For example, if you request a SmartXpert inference on a skull X-ray, there will be no
/// output since SmartXpert doesn't support skull X-rays.
pub async fn get_with_url(
    env: &MilvueUrl,
    key: &str,
    study_id: &str,
    milvue_params: &MilvueParams,
) -> Result<Option<Vec<FileDicomObject<InMemDicomObject>>>, MilvueError> {
    let milvue_api_url = format!("{}/v3/studies/{}", MilvueUrl::get_url(env)?, study_id);

    let mut headers = header::HeaderMap::new();

    let mut api_header = header::HeaderValue::from_str(key)?;
    api_header.set_sensitive(true);
    headers.insert("x-goog-meta-owner", api_header);
    headers.insert("Accept", header::HeaderValue::from_str("application/json")?);

    let client = Client::builder().default_headers(headers).build()?;

    let response = client
        .get(milvue_api_url)
        .query(milvue_params.to_query_param().as_slice())
        .send()
        .await?;

    let content_type = match response.headers().get(reqwest::header::CONTENT_TYPE) {
        Some(content_type) => content_type.to_str()?,
        None => return Err(MilvueError::NoContentType),
    };

    let boundary = match multer::parse_boundary(content_type) {
        Ok(boundary) => boundary,
        Err(err) => match err {
            multer::Error::NoBoundary => {
                return Ok(None);
            }
            _ => return Err(MilvueError::MulterError(err)),
        },
    };

    let body = response.bytes().await?;
    let cursor = Cursor::new(body);
    let mut multipart = Multipart::with_reader(cursor, boundary);

    let mut dicoms = Vec::new();

    while let Some(field) = multipart.next_field().await? {
        let file_bytes = field.bytes().await?;

        match OpenFileOptions::new()
            .read_preamble(ReadPreamble::Always) // Required option since Milvue sends files with a preamble
            .from_reader(Cursor::new(file_bytes.clone()))
        {
            Ok(dicom_file) => {
                dicoms.push(dicom_file);
            }
            Err(err) => return Err(MilvueError::DicomObjectError(err)),
        }
    }

    Ok(Some(dicoms))
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
pub async fn get_study_status(key: &str, study_id: &str) -> Result<reqwest::Response, MilvueError> {
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
) -> Result<reqwest::Response, MilvueError> {
    let milvue_api_url = format!("{}/v3/studies/{}", MilvueUrl::get_url(env)?, study_id);

    let mut headers = header::HeaderMap::new();

    let mut api_header = header::HeaderValue::from_str(key)?;
    api_header.set_sensitive(true);

    headers.insert("x-goog-meta-owner", api_header);
    headers.insert("Accept", header::HeaderValue::from_str("application/json")?);

    let client = Client::builder().default_headers(headers).build()?;

    let response = client
        .get(format!("{}/status", milvue_api_url))
        .send()
        .await?;

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
pub async fn wait_for_done(key: &str, study_id: &str) -> Result<(), MilvueError> {
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
) -> Result<(), MilvueError> {
    let mut status_response = get_study_status_with_url(env, key, study_id).await?;

    let mut status_body: StatusResponse = status_response.json().await?;

    while status_body.status != "done" {
        status_response = get_study_status_with_url(env, key, study_id).await?;
        status_body = status_response.json().await?;
        std::thread::sleep(std::time::Duration::from_secs(3));
    }

    Ok(())
}
