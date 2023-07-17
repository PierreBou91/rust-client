use std::{collections::HashMap, fs, path::PathBuf, process, sync::Arc};

use clap::{Parser, ValueEnum};

use dicom_object::OpenFileOptions;
use milvue_rs::{
    get_with_url, post_stream, wait_for_done_with_url, InferenceCommand, Language, MilvueError,
    MilvueParams, OutputFormat, OutputSelection, RecapTheme, StaticReportFormat,
    StructuredReportFormat,
};
use tokio::sync::{
    mpsc::{self, Sender},
    Barrier,
};
use tracing::{error, info, warn};
use walkdir::WalkDir;

#[derive(Debug)]
struct Event {
    kind: EventKind,
}

#[derive(Debug)]
enum EventKind {
    Uploaded((String, Vec<(String, PathBuf)>)),
    Predicted((String, Vec<(String, PathBuf)>)),
    Downloaded((String, Vec<(String, PathBuf)>)),
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
struct Args {
    /// Input directory
    #[clap(required = true)]
    input_dir: PathBuf,
    /// Output directory
    #[clap(short = 'o', long, default_value = ".")]
    output_dir: PathBuf,
    /// Recursive search in the input directory
    #[clap(short = 'r', long, default_value = "false")]
    recursive: bool,
    /// API key for the Milvue API
    #[clap(short = 'k', long)]
    api_key: String,
    /// API URL for the Milvue API
    #[clap(short, long)]
    api_url: String,
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
    #[clap(short = 's', long, default_value = "rgb")]
    static_report: StaticReportFormat,
    /// Select the format for the structured report
    #[arg(value_enum)]
    #[clap(short = 'S', long, default_value = "none")]
    structured_report: StructuredReportFormat,
    /// Set the log level
    #[arg(value_enum)]
    #[clap(short = 'L', long, default_value = "info")]
    log_level: LogLevel,
    /// Display timestamps with log messages
    #[clap(short = 'T', long)]
    timestamp: bool,
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
async fn main() {
    let args = Args::parse();

    // tracing_subscriber_handler(&args);

    let dicom_list = input_dir_validator(args.clone());

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

    // getting the files to process
    let inventory = match inventory_from_pathbuf(dicom_list) {
        Some(inventory) => inventory,
        None => {
            warn!("No DICOM file to process.");
            return;
        }
    };
    dbg!(inventory.clone());

    let barrier = Arc::new(tokio::sync::Barrier::new(inventory.len()));

    // creating a channel to communicate between the manager and the workers
    // and a vector to store the tasks
    let (tx, mut rx) = mpsc::channel::<Event>(1024);
    let mut tasks = Vec::new();

    // process every study in parallel (in worker threads)
    inventory.clone().into_iter().for_each(|study| {
        let args_clone = args.clone();
        let tx = tx.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            process_study(study, tx, args_clone, barrier).await;
        }))
    });

    // launching a manager thread that will receive the results from the workers
    tasks.push(tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event.kind {
                EventKind::Uploaded(study) => println!("Uploaded: {:?}", study.0),
                EventKind::Predicted(study) => println!("Predicted: {:?}", study.0),
                EventKind::Downloaded(study) => println!("Downloaded: {:?}", study.0),
            }
        }
    }));

    // the end
    drop(tx);
    for task in tasks {
        task.await.unwrap();
    }
}

async fn process_study(
    study: (String, Vec<(String, PathBuf)>),
    tx: Sender<Event>,
    args: Args,
    barrier: Arc<Barrier>,
) {
    println!("Posting study: {:?}", study.clone().0);
    match post_stream(args.api_key.clone(), args.api_url.clone(), study.clone()).await {
        Ok(_) => tx
            .send(Event {
                kind: EventKind::Uploaded(study.clone()),
            })
            .await
            .unwrap(),
        Err(e) => {
            warn!("Error while uploading the study: {}", e);
        }
    };

    // Wait for all studies to be uploaded
    barrier.wait().await;

    // Poll for results
    println!("Polling for results: {:?}", study.clone().0);
    match wait_for_done_with_url(&args.api_url, &args.api_key, &study.0).await {
        Ok(_) => tx
            .send(Event {
                kind: EventKind::Predicted(study.clone()),
            })
            .await
            .unwrap(),
        Err(e) => {
            warn!("Error while polling for results: {}", e);
        }
    };

    // Download the results
    let params = match params_from_args(args.clone()) {
        Ok(params) => params,
        Err(e) => {
            error!("Error: {}", e);
            process::exit(1);
        }
    };

    let mut tasks = Vec::new();

    params.into_iter().for_each(|param| {
        let args_clone = args.clone();
        let study_clone = study.clone();
        let tx = tx.clone();
        match param.inference_command {
            InferenceCommand::SmartUrgences => info!(
                "Downloading SmartUrgences results for study {}",
                study_clone.0
            ),
            InferenceCommand::SmartXpert => {
                info!("Downloading SmartXpert results for study {}", study_clone.0)
            }
        }

        tasks.push(tokio::spawn(async move {
            match get_with_url(
                &args_clone.api_url,
                &args_clone.api_key,
                &study_clone.0,
                &param,
            )
            .await
            {
                Ok(res) => {
                    tx.send(Event {
                        kind: EventKind::Downloaded(study_clone.clone()),
                    })
                    .await
                    .unwrap();
                    match res {
                        Some(dicoms) => {
                            let output_dir = args_clone.output_dir.join(&study_clone.0);
                            if !output_dir.exists() {
                                info!("Creating output directory: {}", output_dir.display());
                                match fs::create_dir_all(&output_dir) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!("Error while creating output directory: {}", e);
                                    }
                                }
                            }

                            for dicom in dicoms {
                                let sop = dicom
                                    .element_by_name("SOPInstanceUID")
                                    .unwrap()
                                    .to_str()
                                    .unwrap();
                                dicom
                                    .write_to_file(format!("{}/{}.dcm", output_dir.display(), sop))
                                    .unwrap();
                            }
                            println!("Saved: {:?}", study_clone.0);
                        }
                        None => {
                            warn!(
                                "No results for study {} for config {:#?}",
                                study_clone.0, param
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!("Error while downloading the results: {}", e);
                }
            };
        }));
    });

    for task in tasks {
        task.await.unwrap();
    }
}

fn params_from_args(args: Args) -> Result<Vec<MilvueParams>, MilvueError> {
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
            structured_report_format: Some(args.structured_report),
            ..Default::default()
        };
        params_list.push(params);
    }
    Ok(params_list)
}

fn input_dir_validator(args: Args) -> Vec<PathBuf> {
    if !args.input_dir.exists() {
        error!(
            "Input directory does not exist: {}",
            args.input_dir.display()
        );
        process::exit(1);
    }

    if !args.input_dir.is_dir() {
        error!(
            "Input directory is not a directory: {}",
            args.input_dir.display()
        );
        process::exit(1);
    }

    let walker = match args.recursive {
        true => WalkDir::new(args.input_dir).into_iter(),
        false => WalkDir::new(args.input_dir).max_depth(1).into_iter(),
    };

    walker
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().is_file() {
                Some(entry.into_path())
            } else {
                None
            }
        })
        .collect()
}

fn inventory_from_pathbuf(dicoms: Vec<PathBuf>) -> Option<HashMap<String, Vec<(String, PathBuf)>>> {
    let mut inventory = HashMap::new();

    // loop over every path provided by the user
    dicoms.iter().for_each(|dicom| {
        // open the file until the PixelData tag
        if let Some(object) = match OpenFileOptions::new()
            .read_until(dicom_dictionary_std::tags::PIXEL_DATA)
            .open_file(dicom)
        {
            Ok(object) => Some(object),
            Err(e) => {
                warn!("{:?} is not a valid dicom file: {}", dicom, e);
                None
            }
        } {
            // Actually build the inventory from the DICOM file
            // TODO: Better error handling to avoid panics in case of bad DICOM file
            let study_instance_uid = object
                .element_by_name("StudyInstanceUID")
                .unwrap_or_else(|_| {
                    panic!(
                        "There should be a StudyInstanceUID element in the DICOM file at {}",
                        dicom.display()
                    )
                })
                .to_str()
                .unwrap_or_else(|_| {
                    panic!(
                        "The StudyInstanceUID element in the DICOM file at {} should be a string",
                        dicom.display()
                    )
                })
                .to_string();

            let sop_instance_uid = object
                .element_by_name("SOPInstanceUID")
                .unwrap_or_else(|_| {
                    panic!(
                        "There should be a SOPInstanceUID element in the DICOM file at {}",
                        dicom.display()
                    )
                })
                .to_str()
                .unwrap_or_else(|_| {
                    panic!(
                        "The SOPInstanceUID element in the DICOM file at {} should be a string",
                        dicom.display()
                    )
                })
                .to_string();

            if sop_instance_uid.contains("1.2.826.0.1.3680043.10.457") {
                warn!("Skipping {}: File is from Milvue", dicom.display());
            }

            inventory
                .entry(study_instance_uid)
                .or_insert_with(Vec::new)
                .push((sop_instance_uid, dicom.to_owned()));
        };
    });

    if inventory.is_empty() {
        None
    } else {
        Some(inventory)
    }
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
