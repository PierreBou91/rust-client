use dicom::object::InMemDicomObject;
use dicom_object::FileDicomObject;
use reqwest::{header, multipart, Client};
use tracing::{debug, error, info};

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
    info!("Preparing POST request for study {}", study_instance_uid);

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
    debug!("Headers: {:?}", headers);

    info!(
        "Building multipart form with {} DICOM files",
        dicom_list.len()
    );
    let form = build_form(dicom_list);

    let client = Client::builder().default_headers(headers).build()?;

    info!("Sending POST request to {}", milvue_api_url);
    let response = client.post(milvue_api_url).multipart(form?).send().await?;

    match response.status() {
        reqwest::StatusCode::OK => info!("POST request successfully sent."),
        status => {
            error!("POST request failed with status code {}", status);
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
fn build_form(
    files: &mut [FileDicomObject<InMemDicomObject>],
) -> Result<multipart::Form, MilvueError> {
    let mut form = multipart::Form::new();
    let number_of_files = files.len();
    for (i, f) in files.iter_mut().enumerate() {
        let mut buffer = Vec::new();
        let instance = f.element_by_name("SOPInstanceUID")?.to_str()?;
        info!(
            "Adding DICOM file {}/{} with SOPInstanceUID {}",
            i + 1,
            number_of_files,
            instance
        );
        f.write_all(&mut buffer)?;
        let part = multipart::Part::bytes(buffer).mime_str("application/dicom")?;
        form = form.part(format!("{}.dcm", instance), part);
    }
    Ok(form)
}
