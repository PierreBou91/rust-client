use std::{env, fmt::Display};

use dicom_object::{FileDicomObject, InMemDicomObject};
use serde::Deserialize;

#[derive(Default)]
pub enum MilvueUrl {
    Dev,
    Staging,
    Prod,
    #[default]
    DefaultUrl,
}

impl MilvueUrl {
    pub fn get_url(&self) -> Result<String, std::env::VarError> {
        match self {
            MilvueUrl::Dev => env::var("MILVUE_API_URL_DEV"),
            MilvueUrl::Staging => env::var("MILVUE_API_URL_STAGING"),
            MilvueUrl::Prod => env::var("MILVUE_API_URL_PROD"),
            MilvueUrl::DefaultUrl => env::var("MILVUE_API_URL"),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct StatusResponse {
    #[serde(rename = "StudyInstanceUID")]
    pub study_instance_uid: String,
    pub status: String,
    pub version: String,
    pub message: String,
}

pub struct MilvueParams {
    pub signed_url: Option<bool>,
    pub output_format: Option<OutputFormat>,
    pub language: Option<Language>,
    pub inference_command: InferenceCommand,
    pub timezone: Option<String>,
    pub output_selection: Option<OutputSelection>,
    pub recap_theme: Option<RecapTheme>,
    pub structured_report_format: Option<StructuredReportFormat>,
    pub static_report_format: Option<StaticReportFormat>,
}

impl MilvueParams {
    pub fn new() -> Self {
        MilvueParams::default()
    }
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

pub enum OutputFormat {
    Overlay,
    Highbit,
    Gsps,
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Overlay => write!(f, "overlay"),
            OutputFormat::Highbit => write!(f, "highbit"),
            OutputFormat::Gsps => write!(f, "gsps"),
        }
    }
}

pub enum Language {
    Fr,
    En,
    Es,
    De,
    It,
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

pub enum InferenceCommand {
    SmartUrgences,
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
pub enum OutputSelection {
    All,
    NoRecap,
    NoNegatives,
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

pub fn check_study_uids(
    dicom_list: &[FileDicomObject<InMemDicomObject>],
) -> Result<String, Box<dyn std::error::Error>> {
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
            return Err(format!(
                "StudyInstanceUID mismatch: expected {}, got {}", // TODO: Improve error message
                study_uid, current_study_uid
            )
            .into());
        }
    }
    Ok(study_uid)
}
