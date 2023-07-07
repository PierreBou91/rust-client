use std::path::PathBuf;

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

    let (sender, mut receiver) = tokio::sync::mpsc::channel(32);

    let sender2 = sender.clone();

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
            manager_sender.send(sop.to_string()).await.unwrap();
        });
        send_tasks.push(send_task);
    }

    for send_task in send_tasks {
        send_task.await.unwrap();
    }

    let parser = tokio::spawn(async move {
        let files = vec!["file1", "file2", "file3"];
        for file in files {
            sender.send(file.to_string()).await.unwrap();
        }
    });

    let parser2 = tokio::spawn(async move {
        let files = vec!["file4", "file5", "file6"];
        for file in files {
            sender2.send(file.to_string()).await.unwrap();
        }
    });

    parser2.await.unwrap();
    parser.await.unwrap();
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
    println!("Loglevel: {:?}", args.log_level);
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
