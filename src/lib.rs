//! # milvue_rs
//! ## Before you begin
//! This library is intended to be used with the Milvue API. To use this library, you must have an API key and the URL of the Milvue
//! environment you wish to use.
//!
//! This key can be obtained by contacting [Milvue](https://www.milvue.com/).
//! Additionally, both of these values must be set as environment variables. More details in the [MilvueUrl] documentation.
//!
//! ## Description
//! **milvue_rs** is a client library in Rust for interacting with the Milvue API, a medical imaging analysis service.
//! This library provides functionality for submitting Digital Imaging and Communications in Medicine (DICOM) files for analysis,
//! and for fetching the resulting annotations. These annotations may include pathology detection or anatomical measurements.
//!
//! The primary interaction with the API involves two steps:
//!
//! 1. Submitting DICOM files for analysis using the [post()] or [post_with_url()] functions.
//! 2. Fetching the resulting analysis using the [get()], [get_study_status()], [wait_for_done()], or [wait_for_done_with_url()] functions.
//!
//! The library provides a variety of structs and enums to support these interactions, including:
//!
//! * [MilvueParams] for specifying the parameters of the request.
//! * [MilvueUrl] for specifying the URL of the Milvue environment to interact with.
//! * [InferenceCommand], [Language], [OutputFormat], [OutputSelection], [RecapTheme], [StaticReportFormat], and [StructuredReportFormat]
//!   for customizing various aspects of the analysis.
//! * [StatusResponse] for representing the response from the Milvue API.
//!
//! Additionally, the [check_study_uids()] function is provided to ensure that all DICOM files submitted in a single request have
//! the same StudyInstanceUID.
//!
//! This library aims to make it easy to integrate the Milvue medical imaging analysis service into Rust applications.
//!
//! ## Example
//! The following example demonstrates how to submit a request to the Milvue API and fetch the resulting analysis.
//! ```rust no_run
//! use dicom_object::open_file;
//! use std::env;
//!
//! #[tokio::main]
//! pub async fn main() {
//!     let key = env::var("MILVUE_API_KEY").unwrap();
//!     let dicom_list = vec![
//!         open_file("DX000000.dcm").unwrap(),
//!     ];
//!
//!     let study_instance_uid = milvue_rs::check_study_uids(&dicom_list).unwrap();
//!     println!("Study Instance UID: {}", study_instance_uid);
//!
//!     let post_response = match milvue_rs::post(&key, &dicom_list).await {
//!         Ok(res) => res,
//!         Err(e) => panic!("Error: {}", e),
//!     };
//!
//!     match post_response.status() {
//!         reqwest::StatusCode::OK => println!("Success!"),
//!         status => println!("Expected status 200 got: {:#?}", status),
//!     }
//!
//!     match milvue_rs::wait_for_done(&key, &study_instance_uid).await {
//!         Ok(_) => println!("Done!"),
//!         Err(e) => panic!("Error: {}", e),
//!     }
//!
//!     let params = milvue_rs::MilvueParams {
//!         language: Some(milvue_rs::Language::En),
//!         ..Default::default()
//!     };
//!
//!     let res = milvue_rs::get(&key, &study_instance_uid, &params)
//!         .await
//!         .unwrap();
//!     for (i, dicom_file) in res.iter().enumerate() {
//!         dicom_file.write_to_file(format!("file{}.dcm", i)).unwrap();
//!     }
//! }
//! ```

mod get;
mod post;
mod structs;

pub use get::{get, get_study_status, wait_for_done, wait_for_done_with_url};
pub use post::{post, post_with_url};
pub use structs::{
    check_study_uids, InferenceCommand, Language, MilvueParams, MilvueUrl, OutputFormat,
    OutputSelection, RecapTheme, StaticReportFormat, StatusResponse, StructuredReportFormat,
};
