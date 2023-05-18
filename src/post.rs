use dicom::object::InMemDicomObject;
use dicom_object::FileDicomObject;
use reqwest::{header, multipart, Client};

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
    dicom_list: &[FileDicomObject<InMemDicomObject>],
) -> Result<reqwest::Response, MilvueError> {
    post_with_url(&MilvueUrl::default(), key, dicom_list).await
}

/// Sends a POST request to upload DICOM files to a specific URL.
///
/// # Arguments
///
/// * `env` - A reference to MilvueUrl that specifies the environment.
/// * `key` - A string slice that holds the API key.
/// * `dicom_list` - A list of DICOM files to be uploaded.
///
/// # Returns
///
/// * A Result wrapping a reqwest::Response indicating the HTTP response or an error.
pub async fn post_with_url(
    env: &MilvueUrl,
    key: &str,
    dicom_list: &[FileDicomObject<InMemDicomObject>],
) -> Result<reqwest::Response, MilvueError> {
    let milvue_api_url = format!("{}/v3/studies", MilvueUrl::get_url(env)?);
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

    let response = client.post(milvue_api_url).multipart(form?).send().await?;

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

    for (i, dicom_file) in files.iter().enumerate() {
        let mut buffer = Vec::new();
        dicom_file.write_all(&mut buffer)?;
        let part = multipart::Part::bytes(buffer)
            .file_name(format!("file{}.dcm", i)) // TODO: Add possibility to get the file name
            .mime_str("application/dicom")?;
        form = form.part(format!("file{}", i), part);
    }

    Ok(form)
}
