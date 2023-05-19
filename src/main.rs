// use dicom_object::open_file;
use std::{env, path::PathBuf, process};

use clap::{Parser, ValueEnum};
use dicom_object::open_file;
use tracing::{debug, error};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to DICOM file(s)
    #[clap(required = true)]
    dicoms: Vec<PathBuf>,
    /// Log level
    #[arg(value_enum)]
    #[clap(short, long, default_value = "info")]
    log_level: LogLevel,
    /// Logs with timer
    #[clap(short, long)]
    timer: Option<bool>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
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

    print!("{:?}", args);
    debug!("args: {:?}", args);

    // let key = env::var("MILVUE_API_KEY").unwrap();

    // let mut dicom_list = Vec::new();
    // for file in args.dicoms {
    //     match open_file(&file) {
    //         Ok(dicom_file) => dicom_list.push(dicom_file),
    //         Err(e) => {
    //             error!("Error: {}", e);
    //             process::exit(1);
    //         }
    //     }
    // }
    // let study_instance_uid = milvue_rs::check_study_uids(&dicom_list).unwrap();

    // match milvue_rs::post(&key, &mut dicom_list).await {
    //     Ok(res) => res,
    //     Err(e) => panic!("Error: {}", e),
    // };

    // // match post_response.status() {
    // //     reqwest::StatusCode::OK => println!("Success!"),
    // //     status => println!("Expected status 200 got: {:#?}", status),
    // // }

    // match milvue_rs::wait_for_done(&key, &study_instance_uid).await {
    //     Ok(_) => {}
    //     Err(e) => panic!("Error: {}", e),
    // }

    // let params = milvue_rs::MilvueParams {
    //     language: Some(milvue_rs::Language::En),
    //     ..Default::default()
    // };

    // let dicoms = match milvue_rs::get(&key, &study_instance_uid, &params).await {
    //     Ok(res) => match res {
    //         Some(d) => d,
    //         None => panic!("No DICOM files found"),
    //     },
    //     Err(e) => panic!("Error: {}", e),
    // };

    // for (i, dicom_file) in dicoms.iter().enumerate() {
    //     dicom_file.write_to_file(format!("file{}.dcm", i)).unwrap();
    // }
}

fn tracing_subscriber_handler(args: &Args) {
    let env_filter = match args.log_level {
        LogLevel::Debug => "milvue_rs=debug",
        LogLevel::Info => "milvue_rs=info",
        LogLevel::Warn => "milvue_rs=warn",
        LogLevel::Error => "milvue_rs=error",
        LogLevel::Quiet => "milvue_rs=off",
    };

    let sub = if let Some(true) = args.timer {
        tracing_subscriber::fmt::Subscriber::builder()
            .with_env_filter(env_filter)
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc3339())
            .finish()
    } else {
        tracing_subscriber::fmt::Subscriber::builder()
            .with_env_filter(env_filter)
            .finish()
    };

    tracing::subscriber::set_global_default(sub)
        .expect("Error while setting subscriber for tracing.");
}
