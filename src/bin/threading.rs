use std::{collections::HashMap, path::PathBuf};

use clap::{Parser, ValueEnum};

use dicom_object::OpenFileOptions;
use tokio::sync::mpsc::{self, Sender};
use tracing::warn;

#[derive(Debug)]
struct Event {
    kind: EventKind,
}

#[derive(Debug)]
enum EventKind {
    Sent(HashMap<String, Vec<String>>),
    // Predicted(String),
    // Downloaded(String),
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to DICOM file(s)
    #[clap(required = true)]
    dicoms: Vec<PathBuf>,
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

    // getting the files to process
    let inventory = match inventory_from_args(args.dicoms.clone()) {
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
    let inventory_clone = inventory.clone();

    inventory_clone.into_iter().for_each(|study| {
        let tx = tx.clone();
        tasks.push(tokio::spawn(async move {
            process_study(study, tx).await;
        }))
    });

    args.dicoms.len(); // ABSOLUTELY REMOVE THIS IT ONLY HERE TO MUTE A WARNING
    inventory.len(); // ABSOLUTELY REMOVE THIS IT ONLY HERE TO MUTE A WARNING

    // launching a manager thread that will receive the results from the workers
    tasks.push(tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event.kind {
                EventKind::Sent(_) => todo!(),
            }
        }
    }));

    // the end
    drop(tx);
    for task in tasks {
        task.await.unwrap();
    }
}

async fn process_study(study: (String, Vec<(String, PathBuf)>), tx: Sender<Event>) {
    println!("processing study: {:?}", study);
    let ev = Event {
        kind: EventKind::Sent(HashMap::new()),
    };
    println!("sending event {:?}", ev);
    println!("with sender {:?}", tx);
}

fn inventory_from_args(dicoms: Vec<PathBuf>) -> Option<HashMap<String, Vec<(String, PathBuf)>>> {
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
