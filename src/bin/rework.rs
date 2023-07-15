use std::{collections::HashMap, path::PathBuf};

use clap::{Parser, ValueEnum};
use dicom_object::{open_file, FileDicomObject, InMemDicomObject};
use tracing::{info, warn};

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

enum Event {
    Sent(HashMap<String, Vec<String>>),
    // Predicted(String),
    // Downloaded(String),
}

#[derive(Debug)]
enum Stage {
    Ready,
    // Sent
    // Predicted,
    // Downloaded,
}

// type Inventory = HashMap<String, Vec<InventoryFile>>;
#[derive(Debug)]
struct InventoryFile {
    sop: String,
    stage: Stage,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber_handler(&args);

    // The DICOM files are loaded in memory
    let dicom_list = match dicom_list_from_args(&args.dicoms) {
        Some(dicom_list) => dicom_list,
        None => {
            warn!("No valid DICOM file found in the dataset.");
            return;
        }
    };

    // Build the inventory
    let mut inventory = HashMap::new();
    for (study_uid, dicom_files) in dicom_list.iter() {
        for f in dicom_files {
            inventory
                .entry(study_uid)
                .or_insert(Vec::new())
                .push(InventoryFile {
                    sop: f
                        .element_by_name("SOPInstanceUID")
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                    stage: Stage::Ready,
                });
        }
    }

    // Main channel to send commands to the manager thread
    let (sender, mut receiver) = tokio::sync::mpsc::channel(128);

    let manager = tokio::spawn(async move {
        while let Some(event) = receiver.recv().await {
            match event {
                Event::Sent(e) => println!("Sent one study \n{:#?}", e),
            }
        }
    });

    let mut send_tasks = Vec::new();

    for (i, study) in dicom_list {
        let manager_sender = sender.clone();
        let send_task = tokio::spawn(async move {
            // TODO: Actually send the files
            manager_sender
                .send(Event::Sent(HashMap::from([(
                    i,
                    study
                        .iter()
                        .map(|f| {
                            f.element_by_name("SOPInstanceUID")
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string()
                        })
                        .collect::<Vec<String>>(),
                )])))
                .await
                .unwrap();
        });
        send_tasks.push(send_task);
    }

    for send_task in send_tasks {
        send_task.await.unwrap();
    }

    drop(sender);
    manager.await.unwrap();
}

type DicomByStudy = HashMap<String, Vec<FileDicomObject<InMemDicomObject>>>;

fn dicom_list_from_args(dicoms: &Vec<PathBuf>) -> Option<DicomByStudy> {
    let mut dicom_list = HashMap::new();
    for file in dicoms {
        match open_file(file) {
            Ok(dicom_file) => {
                info!(
                    "File {} added to the dataset to be analyzed.",
                    file.display()
                );
                dicom_list
                    .entry(
                        dicom_file
                            .element_by_name("StudyInstanceUID")
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string(),
                    )
                    .or_insert_with(Vec::new)
                    .push(dicom_file);
            }
            Err(_) => warn!("Skipping file {}, not a valid dicom file", file.display()),
        }
    }
    if dicom_list.is_empty() {
        return None;
    }
    Some(dicom_list)
}

fn tracing_subscriber_handler(args: &Args) {
    let env_filter = match args.log_level {
        LogLevel::Debug => "rework=debug",
        LogLevel::Info => "rework=info",
        LogLevel::Warn => "rework=warn",
        LogLevel::Error => "rework=error",
        LogLevel::Quiet => "rework=off",
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_dicom_list_from_args() {
        let args = Args {
            dicoms: vec![
                PathBuf::from("src/bin/rework.rs"),
                PathBuf::from("src/bin/rework.rs"),
            ],
            log_level: LogLevel::Info,
            timestamp: false,
        };
        let dicom_list = dicom_list_from_args(&args.dicoms);
        assert!(dicom_list.is_none());
    }

    #[test]
    fn test_dicom_list_from_args2() {
        let args = Args {
            dicoms: vec![PathBuf::from("CR000001.dcm")],
            log_level: LogLevel::Info,
            timestamp: false,
        };
        let dicom_list = dicom_list_from_args(&args.dicoms);
        assert_eq!(1, dicom_list.unwrap().len());
    }
}
