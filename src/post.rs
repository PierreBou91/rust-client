use dicom::object::InMemDicomObject;
use dicom_object::FileDicomObject;
use reqwest::{header, multipart, Client};

use crate::Env;

pub async fn post(
    env: &Env,
    key: &str,
    dicom_list: &[FileDicomObject<InMemDicomObject>],
) -> Result<reqwest::Response, reqwest::Error> {
    let milvue_api_url = Env::get_specific(env);
    let mut headers = header::HeaderMap::new();

    let mut api_header = header::HeaderValue::from_str(key).unwrap();
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

    println!("Building form");
    let form = build_form(dicom_list);

    println!("Building client");
    let client = Client::builder()
        .default_headers(headers)
        // .https_only(true)
        .build()
        .unwrap();

    println!("Sending request");
    let response = client.post(milvue_api_url).multipart(form).send().await?;

    println!("Response {:#?}", response);

    Ok(response)
}

fn build_form(files: &[FileDicomObject<InMemDicomObject>]) -> multipart::Form {
    let mut form = multipart::Form::new();

    for (i, dicom_file) in files.iter().enumerate() {
        // let sop_instance = dicom_file
        //     .element_by_name("SOPInstanceUID")
        //     .unwrap()
        //     .to_str()
        //     .unwrap()
        let mut buffer = Vec::new();
        dicom_file.write_all(&mut buffer).unwrap();
        let part = multipart::Part::bytes(buffer)
            // .file_name(sop_instance)
            .file_name(format!("file{}.dcm", i))
            .mime_str("application/dicom")
            .unwrap();
        form = form.part(format!("file{}", i), part);
    }
    form
}
