# milvue_rs

`milvue_rs` is a Rust client library for the Milvue API, providing the ability to submit Digital Imaging and Communications in Medicine (DICOM) files for analysis and fetch the resulting annotations. These annotations can include pathology detection or anatomical measurements.

## Prerequisites

This library is designed to interact with the Milvue API. In order to use this library, you will need an API key and the URL of the Milvue environment you wish to use. Both of these values must be set as environment variables.

You can obtain your API key by contacting [Milvue](https://www.milvue.com/).

## Dependencies

The `milvue_rs` crate relies on the [dicom-rs](https://github.com/Enet4/dicom-rs) project, a pure Rust implementation of core DICOM standards.

## Example

Below is an example of how to use the `milvue_rs` crate:

```rust
use dicom_object::open_file;
use std::env;

#[tokio::main]
pub async fn main() {
    let key = env::var("MILVUE_API_KEY").unwrap();
    let dicom_list = vec![
        open_file("DX000000.dcm").unwrap(),
    ];

    let study_instance_uid = dicom_rs::check_study_uids(&dicom_list).unwrap();
    println!("Study Instance UID: {}", study_instance_uid);

    let post_response = match dicom_rs::post(&key, &dicom_list).await {
        Ok(res) => res,
        Err(e) => panic!("Error: {}", e),
    };

    match post_response.status() {
        reqwest::StatusCode::OK => println!("Success!"),
        status => println!("Expected status 200 got: {:#?}", status),
    }

    match dicom_rs::wait_for_done(&key, &study_instance_uid).await {
        Ok(_) => println!("Done!"),
        Err(e) => panic!("Error: {}", e),
    }

    let params = dicom_rs::MilvueParams {
        language: Some(dicom_rs::Language::En),
        ..Default::default()
    };

    let res = dicom_rs::get(&key, &study_instance_uid, &params)
        .await
        .unwrap();
    for (i, dicom_file) in res.iter().enumerate() {
        dicom_file.write_to_file(format!("file{}.dcm", i)).unwrap();
    }
}
```

## Support

If you encounter any issues or have inquiries, you can submit them through Github or email support@milvue.com.
