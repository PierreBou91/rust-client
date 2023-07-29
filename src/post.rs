use dicom::object::InMemDicomObject;
use dicom_object::FileDicomObject;
use reqwest::{header, multipart, Body, Client};
use std::path::PathBuf;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::{debug, error};

use crate::{structs::MilvueError, MilvueUrl};

/// Sends a POST request to upload DICOM files in the default environment.
///
/// # Arguments
///
/// * `key` - A string slice that holds the API key.
/// * `dicom_list` - A list of DICOM files to be uploaded.
///
/// # Returns
///
/// * A Result wrapping a reqwest::Response indicating the HTTP response or an error.
pub async fn post(
    key: &str,
    dicom_list: &mut [FileDicomObject<InMemDicomObject>],
) -> Result<reqwest::Response, MilvueError> {
    post_with_url(&MilvueUrl::default().get_url_from_envar()?, key, dicom_list).await
}

/// Sends a POST request to upload DICOM files to a specific URL.
///
/// # Arguments
///
/// * `url` - A reference to MilvueUrl that specifies the environment.
/// * `key` - A string slice that holds the API key.
/// * `dicom_list` - A list of DICOM files to be uploaded.
///
/// # Returns
///
/// * A Result wrapping a reqwest::Response indicating the HTTP response or an error.
pub async fn post_with_url(
    url: &str,
    key: &str,
    dicom_list: &mut [FileDicomObject<InMemDicomObject>],
) -> Result<reqwest::Response, MilvueError> {
    let study_instance_uid = dicom_list[0]
        .element_by_name("StudyInstanceUID")?
        .to_str()?;

    let milvue_api_url = format!("{}/v3/studies", url);

    let mut headers = header::HeaderMap::new();

    let mut api_key = header::HeaderValue::from_str(key)?;
    api_key.set_sensitive(true);

    headers.insert("x-goog-meta-owner", api_key);

    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("multipart/related"),
    );

    headers.insert(
        "type",
        header::HeaderValue::from_static("application/dicom"),
    );

    let form = build_form(dicom_list);

    let client = Client::builder().default_headers(headers).build()?;

    let start = std::time::Instant::now();

    let response = client.post(milvue_api_url).multipart(form?).send().await?;

    debug!("Time to post studies : {:?}", start.elapsed());

    match response.status() {
        reqwest::StatusCode::OK => debug!("Study {} successfully sent.", study_instance_uid),
        status => {
            error!(
                "Error {} while posting study {}",
                status, study_instance_uid
            );
            return Err(MilvueError::StatusResponseError(response));
        }
    }

    Ok(response)
}

/// Sends a POST request to stream DICOM files to a specific URL.
///
/// The method works by building a multipart form of the DICOM files and streaming
/// them to a specified URL. Each file is read as a byte stream which is
/// encapsulated in a multipart form. The status of the upload is checked after the
/// streaming is completed, with successful operations returning a status code of
/// reqwest::StatusCode::OK.
///
/// This method is especially useful for larger DICOM files as it streams the files
/// instead of loading them into memory.
///
/// # Arguments
///
/// * `key` - A string slice that holds the API key.
/// * `url` - A reference to the URL to which the DICOM files will be posted.
/// * `study` - A tuple containing a string representing the study identifier and
/// a vector of tuples, each containing a string representing the SOPInstanceUID
/// and a PathBuf representing the file path of the DICOM file.
///
/// # Returns
///
/// * A Result wrapping a reqwest::Response indicating the HTTP response or an error.
///
/// # Errors
///
/// This function will return an error if the file cannot be opened,
/// if the POST request fails, or if the server returns a non-OK HTTP status code.
pub async fn post_stream(
    key: &str,
    url: &str,
    study: &(String, Vec<(String, PathBuf)>),
) -> Result<reqwest::Response, MilvueError> {
    let milvue_api_url = format!("{}/v3/studies", url);

    let mut headers = header::HeaderMap::new();

    let mut api_key = header::HeaderValue::from_str(key)?;

    api_key.set_sensitive(true);

    headers.insert("x-goog-meta-owner", api_key);

    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("multipart/related"),
    );

    headers.insert(
        "type",
        header::HeaderValue::from_static("application/dicom"),
    );

    let client = Client::builder().default_headers(headers).build()?;

    let mut form = multipart::Form::new();

    for (sop, path) in &study.1 {
        let file = File::open(path).await?;
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = Body::wrap_stream(stream);
        let part = multipart::Part::stream(body).mime_str("application/dicom")?;
        form = form.part(sop.clone(), part);
    }

    let start = std::time::Instant::now();

    let response = client.post(milvue_api_url).multipart(form).send().await?;

    debug!("Time to post study {} : {:?}", study.0, start.elapsed());

    match response.status() {
        reqwest::StatusCode::OK => debug!("Study {} successfully sent.", study.0),
        status => {
            error!("Error {} while posting study {}", status, study.0);
            return Err(MilvueError::StatusResponseError(response));
        }
    }

    Ok(response)
}

/// Builds a multipart form with the provided list of DICOM files.
///
/// # Arguments
///
/// * `files` - A list of DICOM files to be included in the form.
///
/// # Returns
///
/// * A multipart::Form containing all the provided DICOM files.
fn build_form(files: &[FileDicomObject<InMemDicomObject>]) -> Result<multipart::Form, MilvueError> {
    let mut form = multipart::Form::new();

    for f in files.iter() {
        let mut buffer = Vec::new();
        let instance = f.element_by_name("SOPInstanceUID")?.to_str()?;
        f.write_all(&mut buffer)?;
        let part = multipart::Part::bytes(buffer).mime_str("application/dicom")?;
        form = form.part(format!("{}.dcm", instance), part);
    }
    Ok(form)
}
