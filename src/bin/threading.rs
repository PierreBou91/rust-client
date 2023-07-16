use std::{collections::HashMap, path::PathBuf, process};

use clap::{Parser, ValueEnum};

use dicom_object::OpenFileOptions;
use milvue_rs::{post_stream, wait_for_done_with_url};
use tokio::sync::mpsc::{self, Sender};
use tracing::{error, warn};
use walkdir::WalkDir;

#[derive(Debug)]
struct Event {
    kind: EventKind,
}

#[derive(Debug)]
enum EventKind {
    Uploaded((String, Vec<(String, PathBuf)>)),
    Predicted((String, Vec<(String, PathBuf)>)),
    // Downloaded(String),
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
struct Args {
    /// Input directory
    #[clap(required = true)]
    input_dir: PathBuf,
    /// Recursive search in the input directory
    #[clap(short = 'r', long, default_value = "false")]
    recursive: bool,
    /// API key for the Milvue API
    #[clap(short = 'k', long)]
    api_key: String,
    /// API URL for the Milvue API
    #[clap(short, long)]
    api_url: String,
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

    tracing_subscriber_handler(&args);

    // Check that the input directory exists and is a directory
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
        true => WalkDir::new(args.input_dir.clone()).into_iter(),
        false => WalkDir::new(args.input_dir.clone())
            .max_depth(1)
            .into_iter(),
    };
    let dicom_list: Vec<PathBuf> = walker
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().is_file() {
                Some(entry.into_path())
            } else {
                None
            }
        })
        .collect();

    println!("dicom_list: {:#?}", dicom_list);

    // getting the files to process
    let inventory = match inventory_from_pathbuf(dicom_list) {
        Some(inventory) => inventory,
        None => {
            warn!("No DICOM file to process.");
            return;
        }
    };

    println!("inventory: {:#?}", inventory);

    // creating a channel to communicate between the manager and the workers
    // and a vector to store the tasks
    let (tx, mut rx) = mpsc::channel::<Event>(32);
    let mut tasks = Vec::new();

    // process every study in parallel (in worker threads)

    inventory.clone().into_iter().for_each(|study| {
        let args_clone = args.clone();
        let tx = tx.clone();
        tasks.push(tokio::spawn(async move {
            process_study(study, tx, args_clone).await;
        }))
    });

    // launching a manager thread that will receive the results from the workers
    tasks.push(tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event.kind {
                EventKind::Uploaded(study) => println!("Uploaded: {:?}", study),
                EventKind::Predicted(study) => println!("Predicted: {:?}", study),
            }
        }
    }));

    // the end
    drop(tx);
    for task in tasks {
        task.await.unwrap();
    }
}

async fn process_study(study: (String, Vec<(String, PathBuf)>), tx: Sender<Event>, args: Args) {
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

    // Poll for results
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
        LogLevel::Debug => "threading=debug",
        LogLevel::Info => "threading=info",
        LogLevel::Warn => "threading=warn",
        LogLevel::Error => "threading=error",
        LogLevel::Quiet => "threading=off",
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
