use clap::ValueEnum;
use reqwest::{header, Response};
use std::{env, fmt::Display};
use thiserror::Error;

use dicom_object::{FileDicomObject, InMemDicomObject};
use serde::Deserialize;

/// Represents errors that can occur within the `milvue_rs` library.
#[derive(Error, Debug)]
pub enum MilvueError {
    /// Error occurred while creating an HTTP header.
    ///
    /// Typically triggered when a value added to a header violates HTTP specifications.
    #[error("Header creation error: {0}")]
    HeaderCreationError(#[from] header::InvalidHeaderValue),

    /// Network request error.
    ///
    /// Typically triggered when a network request fails (e.g., failed connection, timeout).
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    /// Error occurred when an expected environment variable is not found.
    ///
    /// Typically triggered when the library tries to read an unset environment variable.
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),

    /// No content type found in a response header.
    ///
    /// Typically triggered when a response from the Milvue API lacks a Content-Type header.
    #[error("No content type in Milvue response header. This is likely an error with the Milvue API, please contact support@milvue for assistance.")]
    NoContentType,

    /// Error occurred when converting a header value to a string.
    ///
    /// Typically triggered when a header value contains non-ASCII characters.
    #[error("Error parsing a header element to a string: {0}")]
    ToStringError(#[from] header::ToStrError),

    /// Error occurred when handling multipart form data.
    ///
    /// Typically triggered when parsing multipart form data from a Milvue API response fails.
    #[error("Multer error. The Multer crate, primarily used to fetch the multipart response within milvue_rs, may be the source of this error: {0}")]
    MulterError(#[from] multer::Error),

    /// Error occurred when working with DICOM objects.
    ///
    /// Typically triggered when reading a DICOM file or manipulating a DICOM object fails.
    #[error("Error with the DICOM object crate: {0}")]
    DicomObjectError(#[from] dicom_object::Error),

    /// Error occurred when casting a DICOM value.
    ///
    /// Typically triggered when trying to cast a DICOM value to an incompatible type.
    #[error("Error casting a value with the DICOM crate: {0}")]
    DicomCastError(#[from] dicom::core::value::CastValueError),

    /// HTTP response has an unexpected status.
    ///
    /// Typically triggered when the Milvue API returns a non-successful HTTP status code.
    #[error("Status response error: {0:?}")]
    StatusResponseError(Response),

    /// Uploaded DICOM files do not all belong to the same study.
    ///
    /// Typically triggered when trying to upload DICOM files with different Study Instance UIDs.
    #[error("More than one Study Instance UID found among files to be uploaded.")]
    StudyUidMismatch,

    /// No inference command provided.
    #[error("No inference command provided.")]
    NoInferenceCommand,
}

/// Enum representing possible Milvue URLs.
#[derive(Default)]
pub enum MilvueUrl {
    /// Development environment. Must be set as an environment variable with the key MILVUE_API_URL_DEV.
    Dev,
    /// Staging environment. Must be set as an environment variable with the key MILVUE_API_URL_STAGING.
    Staging,
    /// Production environment. Must be set as an environment variable with the key MILVUE_API_URL_PROD.
    Prod,
    /// Default environment. Must be set as an environment variable with the key MILVUE_API_URL.
    #[default]
    DefaultUrl,
}

/// Provides method to get the URL associated with each enum variant.
impl MilvueUrl {
    /// Gets the environment variable corresponding to the current enum variant.
    ///
    /// # Returns
    ///
    /// * A Result wrapping a String representation of the URL, or an error if the environment variable does not exist.
    pub fn get_url_from_envar(&self) -> Result<String, MilvueError> {
        match self {
            MilvueUrl::Dev => env::var("MILVUE_API_URL_DEV")
                .map_err(|_| MilvueError::EnvVarNotFound("MILVUE_API_URL_DEV".into())),
            MilvueUrl::Staging => env::var("MILVUE_API_URL_STAGING")
                .map_err(|_| MilvueError::EnvVarNotFound("MILVUE_API_URL_STAGING".into())),
            MilvueUrl::Prod => env::var("MILVUE_API_URL_PROD")
                .map_err(|_| MilvueError::EnvVarNotFound("MILVUE_API_URL_PROD".into())),
            MilvueUrl::DefaultUrl => env::var("MILVUE_API_URL")
                .map_err(|_| MilvueError::EnvVarNotFound("MILVUE_API_URL".into())),
        }
    }
}

/// Represents the response status for the [crate::get::wait_for_done()] function.
#[derive(Deserialize, Debug)]
pub struct StatusResponse {
    #[serde(rename = "StudyInstanceUID")]
    pub study_instance_uid: String,
    pub status: String,
    pub version: String,
    pub message: String,
}

#[derive(Debug, Clone)]
/// Represents the parameters to configure the Milvue request.
pub struct MilvueParams {
    /// Whether or not to return a signed URL to handle the DICOM files instead of downloading them directly.
    pub signed_url: Option<bool>,
    /// [OutputFormat]
    pub output_format: Option<OutputFormat>,
    /// [Language]
    pub language: Option<Language>,
    /// [InferenceCommand]
    pub inference_command: InferenceCommand,
    /// The timezone delay from UTC in hours. For example, if the timezone is UTC+2, the value should be +2.
    pub timezone: Option<String>,
    /// [OutputSelection]
    pub output_selection: Option<OutputSelection>,
    /// [RecapTheme]
    pub recap_theme: Option<RecapTheme>,
    /// [StructuredReportFormat]
    pub structured_report_format: Option<StructuredReportFormat>,
    /// [StaticReportFormat]
    pub static_report_format: Option<StaticReportFormat>,
}

impl MilvueParams {
    pub fn new() -> Self {
        MilvueParams::default()
    }

    /// Method to convert the MilvueParams struct into a Vec of query parameters.
    pub fn to_query_param(&self) -> Vec<(String, String)> {
        let mut query_params: Vec<(String, String)> = vec![];
        if let Some(signed_url) = self.signed_url {
            query_params.push(("signed_url".to_string(), signed_url.to_string()));
        }
        if let Some(output_format) = &self.output_format {
            query_params.push(("output_format".to_string(), output_format.to_string()));
        }
        if let Some(language) = &self.language {
            query_params.push(("language".to_string(), language.to_string()));
        }
        query_params.push((
            "inference_command".to_string(),
            self.inference_command.to_string(),
        ));
        if let Some(timezone) = &self.timezone {
            query_params.push(("timezone".to_string(), timezone.to_string()));
        }
        if let Some(output_selection) = &self.output_selection {
            query_params.push(("output_selection".to_string(), output_selection.to_string()));
        }
        if let Some(recap_theme) = &self.recap_theme {
            query_params.push(("recap_theme".to_string(), recap_theme.to_string()));
        }
        if let Some(structured_report_format) = &self.structured_report_format {
            query_params.push((
                "structured_report_format".to_string(),
                structured_report_format.to_string(),
            ));
        }
        if let Some(static_report_format) = &self.static_report_format {
            query_params.push((
                "static_report_format".to_string(),
                static_report_format.to_string(),
            ));
        }

        query_params
    }
}

impl Default for MilvueParams {
    fn default() -> Self {
        MilvueParams {
            signed_url: None,
            output_format: Some(OutputFormat::Overlay),
            language: Some(Language::Fr),
            inference_command: InferenceCommand::SmartUrgences,
            timezone: None,
            output_selection: Some(OutputSelection::All),
            recap_theme: Some(RecapTheme::Dark),
            structured_report_format: None,
            static_report_format: Some(StaticReportFormat::Rgb),
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
/// Represents the output format expected from the Milvue API.
pub enum OutputFormat {
    /// Overlay contains a copy of the original image with the annotations in a separate dicom tag.
    Overlay,
    /// Highbit contains a copy of the original image with the annotations burnt into the original pixel array.
    Highbit,
    /// Gsps is a mask that displays on top of the original image.
    Gsps,
    /// SC
    SecondaryCapture,
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Overlay => write!(f, "overlay"),
            OutputFormat::Highbit => write!(f, "highbit"),
            OutputFormat::Gsps => write!(f, "gsps"),
            OutputFormat::SecondaryCapture => write!(f, "secondary_capture"),
        }
    }
}

/// Represents the language of the annotations.
#[derive(Clone, Debug, ValueEnum)]
pub enum Language {
    /// French
    Fr,
    /// English
    En,
    /// Spanish
    Es,
    /// German
    De,
    /// Italian
    It,
    /// Portuguese
    Pt,
}

impl Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Fr => write!(f, "fr"),
            Language::En => write!(f, "en"),
            Language::Es => write!(f, "es"),
            Language::De => write!(f, "de"),
            Language::It => write!(f, "it"),
            Language::Pt => write!(f, "pt"),
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
/// Represents the inference command for the Milvue request.
pub enum InferenceCommand {
    /// SmartUrgences yields the pathology detection.
    SmartUrgences,
    /// SmartXpert yields the anatomical measurements.
    SmartXpert,
}

impl Display for InferenceCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InferenceCommand::SmartUrgences => write!(f, "smarturgences"),
            InferenceCommand::SmartXpert => write!(f, "smartxpert"),
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
/// Represents the output selection for the Milvue request.
pub enum OutputSelection {
    /// All contains all the possible outputs including negatives and out of scope results.
    All,
    /// NoRecap contains all the possible outputs except the recap.
    NoRecap,
    /// NoNegatives contains all the possible outputs except the negatives and out of scope results.
    NoNegatives,
    /// None contains no output.
    None,
}

impl Display for OutputSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputSelection::All => write!(f, "all"),
            OutputSelection::NoRecap => write!(f, "no_recap"),
            OutputSelection::NoNegatives => write!(f, "no_negatives"),
            OutputSelection::None => write!(f, "none"),
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
/// Represents the recap theme for the Milvue request.
pub enum RecapTheme {
    Dark,
    Light,
}

impl Display for RecapTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecapTheme::Dark => write!(f, "dark"),
            RecapTheme::Light => write!(f, "light"),
        }
    }
}

/// Represents the structured report format for the Milvue request.
///
/// If set, this parameter will return a structured report in the specified format.
#[derive(Clone, Debug, ValueEnum)]
pub enum StructuredReportFormat {
    Lite,
    Normal,
    Full,
    None,
}

impl Display for StructuredReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StructuredReportFormat::Lite => write!(f, "lite"),
            StructuredReportFormat::Normal => write!(f, "normal"),
            StructuredReportFormat::Full => write!(f, "full"),
            StructuredReportFormat::None => write!(f, "none"),
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
/// Represents the static report format for the Milvue request.
pub enum StaticReportFormat {
    Rgb,
    Pdf,
    None,
}

impl Display for StaticReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StaticReportFormat::Rgb => write!(f, "rgb"),
            StaticReportFormat::Pdf => write!(f, "pdf"),
            StaticReportFormat::None => write!(f, "none"),
        }
    }
}

/// Checks if all DICOM files in the provided list have the same StudyInstanceUID.
///
/// # Arguments
///
/// * `dicom_list` - A list of DICOM files to be checked.
///
/// # Returns
///
/// * A Result wrapping a String representation of the StudyInstanceUID if all DICOM files have the same StudyInstanceUID,
/// or an error if there is a mismatch.
pub fn check_study_uids(
    dicom_list: &[FileDicomObject<InMemDicomObject>],
) -> Result<String, MilvueError> {
    let study_uid = dicom_list[0]
        .element_by_name("StudyInstanceUID")?
        .to_str()?
        .to_string();
    for dicom in dicom_list {
        let current_study_uid = dicom
            .element_by_name("StudyInstanceUID")?
            .to_str()?
            .to_string();
        if study_uid != current_study_uid {
            return Err(MilvueError::StudyUidMismatch);
        }
    }
    Ok(study_uid)
}
