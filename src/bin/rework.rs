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
    Sent(String),
    Predicted(String),
    Downloaded(String),
}

enum Stage {
    Sent,
    Predicted,
    Downloaded,
}

type Inventory = HashMap<String, Vec<InventoryFile>>;

struct InventoryFile {
    sop: String,
    stage: Stage,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber_handler(&args);

    let dicom_list = match dicom_list_from_args(&args.dicoms) {
        Some(dicom_list) => dicom_list,
        None => {
            warn!("No valid DICOM file found in the dataset.");
            return;
        }
    };

    // Main channel to send commands to the manager thread
    let (sender, mut receiver) = tokio::sync::mpsc::channel(32);

    // let sender2 = sender.clone();

    let manager = tokio::spawn(async move {
        while let Some(file) = receiver.recv().await {
            println!("Received file: {}", file);
        }
    });

    let mut send_tasks = Vec::new();

    for dicom in dicom_list {
        let manager_sender = sender.clone();
        let send_task = tokio::spawn(async move {
            let sop = dicom
                .element_by_name("SOPInstanceUID")
                .unwrap()
                .to_str()
                .unwrap();
            println!("Sending file: {}", sop);
            if sop == "1.2.276.0.7230010.3.1.4.808989797.1.1677236046.446300" {
                // wait for 1 second
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
            manager_sender.send(sop.to_string()).await.unwrap();
        });
        send_tasks.push(send_task);
    }

    for send_task in send_tasks {
        send_task.await.unwrap();
    }

    // let parser = tokio::spawn(async move {
    //     let files = vec!["file1", "file2", "file3"];
    //     for file in files {
    //         sender.send(file.to_string()).await.unwrap();
    //     }
    // });

    // let parser2 = tokio::spawn(async move {
    //     let files = vec!["file4", "file5", "file6"];
    //     for file in files {
    //         sender2.send(file.to_string()).await.unwrap();
    //     }
    // });

    // parser2.await.unwrap();
    // parser.await.unwrap();
    drop(sender);
    manager.await.unwrap();
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
