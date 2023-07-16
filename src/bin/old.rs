use std::{env, fs, path::PathBuf, process, sync::Arc};

use clap::{Parser, ValueEnum};
use dicom::core::{DataElement, PrimitiveValue, VR};
use dicom_object::{open_file, FileDicomObject, InMemDicomObject, Tag};
use milvue_rs::{
    InferenceCommand, Language, MilvueError, MilvueParams, OutputFormat, OutputSelection,
    RecapTheme, StaticReportFormat, StructuredReportFormat,
};
use num_bigint::BigUint;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to DICOM file(s)
    #[clap(required = true)]
    dicoms: Vec<PathBuf>,
    /// Output directory
    #[clap(short = 'o', long, default_value = ".")]
    output_dir: PathBuf,
    /// Override the API key from the environment variable
    #[clap(short = 'k', long)]
    api_key: Option<String>,
    /// NOT YET IMPLEMENTED USE ENVARS Override the API URL from the environment variable
    #[clap(short, long)]
    api_url: Option<String>,
    /// Run SmartUrgences inference on the dataset
    #[clap(short = 'u', long)]
    smarturgences: bool,
    /// Run SmartXpert inference on the dataset
    #[clap(short = 'x', long)]
    smartxpert: bool,
    /// Set the language for the annotated images
    #[arg(value_enum)]
    #[clap(short, long, default_value = "en")]
    language: Language,
    /// Specify the output format for the annotated images
    #[arg(value_enum)]
    #[clap(short, long, default_value = "overlay")]
    format: OutputFormat,
    /// Choose the output selection
    #[arg(value_enum)]
    #[clap(short = 'O', long, default_value = "all")]
    output_selection: OutputSelection,
    /// Choose the theme for the recap
    #[arg(value_enum)]
    #[clap(short = 't', long, default_value = "dark")]
    recap_theme: RecapTheme,
    /// Select the format for the static report
    #[arg(value_enum)]
    #[clap(short = 'r', long, default_value = "rgb")]
    static_report: StaticReportFormat,
    /// Select the format for the structured report
    #[arg(value_enum)]
    #[clap(short = 'R', long, default_value = "none")]
    structured_report: StructuredReportFormat,
    /// Set the log level
    #[arg(value_enum)]
    #[clap(short = 'L', long, default_value = "info")]
    log_level: LogLevel,
    /// Display timestamps with log messages
    #[clap(short = 'T', long)]
    timestamp: bool,
    // Options to be added when they are implemented in the library:
    // Signed URL
    // Timezone
}

#[derive(Copy, Clone, ValueEnum, Debug)]
enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Quiet,
}

#[tokio::main]
pub async fn main() {
    let args = Args::parse();

    tracing_subscriber_handler(&args);

    let params = match params_from_args(&args) {
        Ok(params) => params,
        Err(e) => {
            error!("Error: {}", e);
            process::exit(1);
        }
    };

    if !args.output_dir.exists() {
        info!("Creating output directory: {}", args.output_dir.display());
        match fs::create_dir_all(&args.output_dir) {
            Ok(_) => {}
            Err(e) => {
                error!("Error while creating output directory: {}", e);
                process::exit(1);
            }
        }
    }

    let mut dicom_list = match dicom_list_from_args(&args.dicoms) {
        Some(dicom_list) => dicom_list,
        None => {
            error!("No valid DICOM files found at the specified path(s), exiting.");
            process::exit(1);
        }
    };

    let study_instance_uid = match milvue_rs::check_study_uids(&dicom_list) {
        Ok(uid) => uid,
        Err(e) => {
            error!("Error: {}", e);
            process::exit(1);
        }
    };

    let new_study_instance_uid = generate_dicom_uid();

    update_study_instance_uid(&mut dicom_list, &new_study_instance_uid);

    let key = match args.api_key {
        Some(key) => key,
        None => match env::var("MILVUE_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                error!("No API key provided, exiting.");
                process::exit(1);
            }
        },
    };

    match milvue_rs::post(&key, &mut dicom_list).await {
        Ok(res) => res,
        Err(e) => {
            error!("Error while posting study: {}", e);
            process::exit(1);
        }
    };

    match milvue_rs::wait_for_done(&key, &new_study_instance_uid).await {
        Ok(_) => {}
        Err(e) => {
            error!("Error while waiting for study to be processed: {}", e);
            process::exit(1);
        }
    }

    // if args.api_url.is_some() {
    //     let api_url = Arc::new(args.api_url);
    // }
    let key = Arc::new(key);
    let new_study_instance_uid = Arc::new(new_study_instance_uid);
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();

    for config in params {
        let key_clone = Arc::clone(&key);
        let uid_clone = Arc::clone(&new_study_instance_uid);
        let results_clone = Arc::clone(&results);

        let handle = tokio::spawn(async move {
            match milvue_rs::get(&key_clone, &uid_clone, &config).await {
                Ok(res) => match res {
                    Some(mut d) => {
                        let mut results = results_clone.lock().await;
                        results.append(&mut d);
                    }
                    None => {
                        warn!("No results for config: {:#?}", config);
                    }
                },
                Err(e) => error!("Error while getting results: {}", e),
            };
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap(); // Fine to unwrap here, thread should not panic unless fatal error.
    }

    let mut results = results.lock().await;

    update_study_instance_uid(&mut results, &study_instance_uid);

    for (i, dicom_file) in results.iter().enumerate() {
        dicom_file
            .write_to_file(format!("{}/file{}.dcm", args.output_dir.display(), i))
            .unwrap();
    }
}

fn dicom_list_from_args(dicoms: &Vec<PathBuf>) -> Option<Vec<FileDicomObject<InMemDicomObject>>> {
    let mut dicom_list = Vec::new();
    for file in dicoms {
        match open_file(file) {
            Ok(dicom_file) => {
                info!(
                    "File {} added to the dataset to be analyzed.",
                    file.display()
                );
                dicom_list.push(dicom_file)
            }
            Err(e) => warn!("Skipping file {}: {}", file.display(), e),
        }
    }
    if dicom_list.is_empty() {
        return None;
    }
    Some(dicom_list)
}

/// Get the parameters from the command line arguments, and return a list of
/// MilvueParams to be used for the inference.
fn params_from_args(args: &Args) -> Result<Vec<MilvueParams>, MilvueError> {
    if !args.smarturgences && !args.smartxpert {
        return Err(MilvueError::NoInferenceCommand);
    }
    let mut params_list = Vec::new();
    if args.smarturgences {
        let params = MilvueParams {
            language: Some(args.language.clone()),
            recap_theme: Some(args.recap_theme.clone()),
            inference_command: InferenceCommand::SmartUrgences,
            output_format: Some(args.format.clone()),
            output_selection: Some(args.output_selection.clone()),
            static_report_format: Some(args.static_report.clone()),
            structured_report_format: Some(args.structured_report.clone()),
            ..Default::default()
        };
        params_list.push(params);
    }
    if args.smartxpert {
        let params = MilvueParams {
            language: Some(args.language.clone()),
            recap_theme: Some(args.recap_theme.clone()),
            output_format: Some(args.format.clone()),
            output_selection: Some(args.output_selection.clone()),
            inference_command: InferenceCommand::SmartXpert,
            static_report_format: Some(args.static_report.clone()),
            structured_report_format: Some(args.structured_report.clone()),
            ..Default::default()
        };
        params_list.push(params);
    }
    Ok(params_list)
}

fn tracing_subscriber_handler(args: &Args) {
    let env_filter = match args.log_level {
        LogLevel::Debug => "milvue_rs=debug",
        LogLevel::Info => "milvue_rs=info",
        LogLevel::Warn => "milvue_rs=warn",
        LogLevel::Error => "milvue_rs=error",
        LogLevel::Quiet => "milvue_rs=off",
    };

    // "if" because the subscriber doesn't yield the same type with or without time wich prevents
    // using a match statement.
    if args.timestamp {
        let sub = tracing_subscriber::fmt::Subscriber::builder()
            .with_env_filter(env_filter)
            .finish();
        tracing::subscriber::set_global_default(sub)
            .expect("Error while setting subscriber for tracing.");
    } else {
        let sub = tracing_subscriber::fmt::Subscriber::builder()
            .with_env_filter(env_filter)
            .without_time()
            .finish();
        tracing::subscriber::set_global_default(sub)
            .expect("Error while setting subscriber for tracing.");
    };
}

// Valid way of generating UID without a dedicated dicom OID
// http://www.dclunie.com/medical-image-faq/html/part2.html#UUID
pub fn generate_dicom_uid() -> String {
    let uuid = Uuid::new_v4();
    let bytes = uuid.as_bytes();
    let bigint = BigUint::from_bytes_le(bytes);

    format!("2.25.{}", bigint)
}

fn update_study_instance_uid(
    dicom_list: &mut Vec<FileDicomObject<InMemDicomObject>>,
    new_study_instance_uid: &str,
) {
    for dicom in dicom_list {
        let new_element = DataElement::new(
            Tag(0x0020, 0x000D),
            VR::UI,
            PrimitiveValue::Str(new_study_instance_uid.to_string()),
        );
        dicom.put(new_element);
    }
}
