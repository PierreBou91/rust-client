use dicom_object::{file::ReadPreamble, FileDicomObject, InMemDicomObject, OpenFileOptions};
use multer::Multipart;
use reqwest::{header, Client};
use std::io::Cursor;
use tracing::{debug, error, info, warn};

use crate::{structs::MilvueError, MilvueParams, MilvueUrl, StatusResponse};

/// Fetches DICOM files from a study in the default environment.
///
/// # Arguments
///
/// * `key` - A string slice that holds the API key
/// * `study_instance_uid` - A string slice that holds the ID of the study
/// * `milvue_params` - A reference to MilvueParams containing parameters for the request
///
/// # Returns
///
/// * An option containing a vector of DICOM files or None, in the case of a None, this means that there is no output for the given
/// configuration. For example, if you request a SmartXpert inference on a skull X-ray, there will be no output since SmartXpert
/// doesn't support skull X-rays.
pub async fn get(
    key: &str,
    study_instance_uid: &str,
    milvue_params: &MilvueParams,
) -> Result<Option<Vec<FileDicomObject<InMemDicomObject>>>, MilvueError> {
    get_with_url(
        &MilvueUrl::default().get_url_from_envar()?,
        key,
        study_instance_uid,
        milvue_params,
    )
    .await
}

/// Fetches DICOM files from a study in the specified environment.
///
/// # Arguments
///
/// * `url` - A reference to MilvueUrl that specifies the environment
/// * `key` - A string slice that holds the API key
/// * `study_instance_uid` - A string slice that holds the ID of the study
/// * `milvue_params` - A reference to MilvueParams containing parameters for the request
///
/// # Returns
///
/// * An Option containing a vector of DICOM files or None, in the case of a None, this means that there is no output
/// for the given configuration. For example, if you request a SmartXpert inference on a skull X-ray, there will be no
/// output since SmartXpert doesn't support skull X-rays.
pub async fn get_with_url(
    url: &str,
    key: &str,
    study_instance_uid: &str,
    milvue_params: &MilvueParams,
) -> Result<Option<Vec<FileDicomObject<InMemDicomObject>>>, MilvueError> {
    info!("Preparing GET request for study {}", study_instance_uid);

    let milvue_api_url = format!("{}/v3/studies/{}", url, study_instance_uid);

    let mut headers = header::HeaderMap::new();

    let mut api_key = header::HeaderValue::from_str(key)?;
    api_key.set_sensitive(true);
    headers.insert("x-goog-meta-owner", api_key);

    headers.insert("Accept", header::HeaderValue::from_str("application/json")?);
    debug!("Headers: {:?}", headers);

    let client = Client::builder().default_headers(headers).build()?;

    info!("Sending GET request to {}", milvue_api_url);
    let response = client
        .get(milvue_api_url)
        .query(milvue_params.to_query_param().as_slice())
        .send()
        .await?;

    match response.status() {
        reqwest::StatusCode::OK => info!("GET request successfully sent."),
        status => {
            error!("GET request failed with status code {}", status);
            return Err(MilvueError::StatusResponseError(response));
        }
    }

    let content_type = match response.headers().get(reqwest::header::CONTENT_TYPE) {
        Some(content_type) => content_type.to_str()?,
        None => return Err(MilvueError::NoContentType),
    };
    debug!("Content-Type: {}", content_type);

    let boundary_parts = content_type.split("boundary=").collect::<Vec<_>>();
    let boundary = match boundary_parts.len() {
        2 => boundary_parts[1].to_string(),
        _ => {
            warn!("No boundary found in Content-Type header, it is likely that the study has no output for the given configuration (inference command, output_selection, etc.)");
            return Ok(None);
        }
    };

    let body = response.bytes().await?;
    let cursor = Cursor::new(body);
    info!("Parsing multipart response");
    let mut multipart = Multipart::with_reader(cursor, boundary);

    let mut dicoms = Vec::new();
    let mut dicom_count = 1;

    while let Some(field) = multipart.next_field().await? {
        let file_bytes = field.bytes().await?;

        match OpenFileOptions::new()
            .read_preamble(ReadPreamble::Always) // Required option since Milvue sends files with a preamble
            .from_reader(Cursor::new(file_bytes.clone()))
        {
            Ok(dicom_file) => {
                info!("DICOM file {} successfully parsed", dicom_count);
                debug!(
                    "SOPInstanceUID: {}",
                    dicom_file.element_by_name("SOPInstanceUID")?.to_str()?
                );
                dicoms.push(dicom_file);
            }
            Err(err) => {
                error!("Error parsing DICOM file {}: {}", dicom_count, err);
                return Err(MilvueError::DicomObjectError(err));
            }
        }
        dicom_count += 1;
    }
    info!("{} DICOM files successfully parsed", dicom_count - 1);
    Ok(Some(dicoms))
}

/// Fetches the status of a study in the default environment.
///
/// # Arguments
///
/// * `key` - A string slice that holds the API key
/// * `study_instance_uid` - A string slice that holds the ID of the study
///
/// # Returns
///
/// * A Result containing the response from the server or an error
pub async fn get_study_status(
    key: &str,
    study_instance_uid: &str,
) -> Result<reqwest::Response, MilvueError> {
    get_study_status_with_url(
        &MilvueUrl::default().get_url_from_envar()?,
        key,
        study_instance_uid,
    )
    .await
}

/// Fetches the status of a study in the specified environment.
///
/// # Arguments
///
/// * `url` - A reference to MilvueUrl that specifies the environment
/// * `key` - A string slice that holds the API key
/// * `study_instance_uid` - A string slice that holds the ID of the study
///
/// # Returns
///
/// * A Result containing the response from the server or an error
pub async fn get_study_status_with_url(
    url: &str,
    key: &str,
    study_instance_uid: &str,
) -> Result<reqwest::Response, MilvueError> {
    let milvue_api_url = format!("{}/v3/studies/{}", url, study_instance_uid);

    let mut headers = header::HeaderMap::new();

    let mut api_header = header::HeaderValue::from_str(key)?;
    api_header.set_sensitive(true);

    headers.insert("x-goog-meta-owner", api_header);
    headers.insert("Accept", header::HeaderValue::from_str("application/json")?);

    let client = Client::builder().default_headers(headers).build()?;

    debug!(
        "Fetching status of study {} from {}",
        study_instance_uid, url
    );
    let response = client
        .get(format!("{}/status", milvue_api_url))
        .send()
        .await?;

    match response.status() {
        reqwest::StatusCode::OK => debug!("GET request successfully sent."),
        status => {
            error!("GET request failed with status code {}", status);
            return Err(MilvueError::StatusResponseError(response));
        }
    }

    Ok(response)
}

/// Waits for a study to be done in the default environment.
///
/// # Arguments
///
/// * `key` - A string slice that holds the API key
/// * `study_instance_uid` - A string slice that holds the ID of the study
///
/// # Returns
///
/// * A Result indicating success (empty Ok value) or an error
pub async fn wait_for_done(key: &str, study_instance_uid: &str) -> Result<(), MilvueError> {
    wait_for_done_with_url(
        &MilvueUrl::default().get_url_from_envar()?,
        key,
        study_instance_uid,
    )
    .await
}

/// Waits for a study to be done in the specified environment.
///
/// # Arguments
///
/// * `url` - A reference to MilvueUrl that specifies the environment
/// * `key` - A string slice that holds the API key
/// * `study_instance_uid` - A string slice that holds the ID of the study
///
/// # Returns
///
/// * A Result indicating success (empty Ok value) or an error
pub async fn wait_for_done_with_url(
    url: &str,
    key: &str,
    study_instance_uid: &str,
) -> Result<(), MilvueError> {
    info!("Waiting for study {} to be done", study_instance_uid);
    let mut status_response = get_study_status_with_url(url, key, study_instance_uid).await?;

    let mut status_body: StatusResponse = status_response.json().await?;

    while status_body.status != "done" {
        let span = tracing::span!(tracing::Level::INFO, "wait_for_done");
        let _enter = span.enter();
        info!(
            "Study {} is not done yet, waiting 3 seconds",
            study_instance_uid
        );
        status_response = get_study_status_with_url(url, key, study_instance_uid).await?;
        status_body = status_response.json().await?;
        std::thread::sleep(std::time::Duration::from_secs(3));
    }

    Ok(())
}
