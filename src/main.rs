use reqwest::{header, multipart, Client};
use std::io::Read;
use std::{env, fs::File};

const MILVUE_API_URL: &str = "redacted/v3/studies";

struct MilvuePart {
    file: File,
    file_name: String,
}

#[tokio::main]
pub async fn main() -> Result<(), reqwest::Error> {
    let milvue_api_key = env::var("MILVUE_API_KEY").unwrap();
    let mut headers = header::HeaderMap::new();

    let mut api_header = header::HeaderValue::from_str(&milvue_api_key).unwrap();
    api_header.set_sensitive(true);
    headers.insert("x-goog-meta-owner", api_header);

    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("multipart/related"),
    );

    headers.insert(
        "type",
        header::HeaderValue::from_static("application/dicom"),
    );

    println!("Loading file");

    let file = File::open("DX000000.dcm").unwrap();
    let file2 = File::open("DX000001.dcm").unwrap();

    let milvue_parts = vec![
        MilvuePart {
            file,
            file_name: "file1".to_string(),
        },
        MilvuePart {
            file: file2,
            file_name: "file2".to_string(),
        },
    ];

    println!("Building form");
    let form = build_form(milvue_parts);

    println!("Building client");
    let client = Client::builder()
        .default_headers(headers)
        // .https_only(true)
        // .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();

    println!("Sending request");
    let response = client.post(MILVUE_API_URL).multipart(form).send().await?;

    println!("Response {:#?}", response);

    Ok(())
}

fn build_form(files: Vec<MilvuePart>) -> multipart::Form {
    let mut form = multipart::Form::new();

    for (i, mut milvue_part) in files.into_iter().enumerate() {
        let mut file_bytes = Vec::new();
        milvue_part
            .file
            .read_to_end(&mut file_bytes)
            .expect("Failed to read file content");

        let part = multipart::Part::bytes(file_bytes)
            .file_name(milvue_part.file_name)
            .mime_str("application/dicom")
            .unwrap();
        form = form.part(format!("file{}", i), part);
    }
    form
}
