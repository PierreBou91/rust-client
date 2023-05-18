// use dicom_object::open_file;
use std::{path::PathBuf, process};

use clap::Parser;
use milvue_rs::MilvueUrl;
use tracing::{debug, error};
use tracing_subscriber::filter::LevelFilter;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to DICOM file(s)
    #[clap(required = true)]
    dicoms: Vec<PathBuf>,
}

#[tokio::main]
pub async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .without_time()
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Error while setting subscriber for tracing.");

    let args = Args::parse();
    debug!("I'm debugging!");
    println!("args: {:?}", args);
    let envar = match MilvueUrl::DefaultUrl.get_url_from_envar() {
        Ok(url) => url,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };
    print!("envar: {}", envar);
    // let key = env::var("MILVUE_API_KEY").unwrap();
    // let dicom_list = vec![
    //     open_file("DX000000.dcm").unwrap(),
    //     // open_file("CR000000.dcm").unwrap(),
    //     // open_file("DX000001.dcm").unwrap(),
    //     // open_file("94026a15-c05d-400a-aaf0-09e96e507648.3af86414-5909-42b5-a68b-2e352bbd9d13.a8b13976-d3cd-459e-a42b-e9d04d7f14dc.dcm").unwrap(),
    //     // open_file("94026a15-c05d-400a-aaf0-09e96e507648.3af86414-5909-42b5-a68b-2e352bbd9d13.c8cd6ff3-7297-4140-9666-7eb36bedc6a3.dcm").unwrap(),
    //     // open_file("94026a15-c05d-400a-aaf0-09e96e507648.3af86414-5909-42b5-a68b-2e352bbd9d13.d99c8948-140d-4b37-a77f-d0db38270623.dcm").unwrap(),
    // ];

    // let study_instance_uid = milvue_rs::check_study_uids(&dicom_list).unwrap();
    // println!("Study Instance UID: {}", study_instance_uid);

    // let post_response = match milvue_rs::post(&key, &dicom_list).await {
    //     Ok(res) => res,
    //     Err(e) => panic!("Error: {}", e),
    // };

    // match post_response.status() {
    //     reqwest::StatusCode::OK => println!("Success!"),
    //     status => println!("Expected status 200 got: {:#?}", status),
    // }

    // match milvue_rs::wait_for_done(&key, &study_instance_uid).await {
    //     Ok(_) => println!("Done!"),
    //     Err(e) => panic!("Error: {}", e),
    // }

    // let params = milvue_rs::MilvueParams {
    //     language: Some(milvue_rs::Language::En),
    //     ..Default::default()
    // };

    // let res = milvue_rs::get(&key, &study_instance_uid, &params)
    //     .await
    //     .unwrap();
    // for (i, dicom_file) in res.iter().enumerate() {
    //     dicom_file.write_to_file(format!("file{}.dcm", i)).unwrap();
    // }
}
