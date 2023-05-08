use dicom_object::open_file;
use std::env;

#[tokio::main]
pub async fn main() {
    let env = milvue_rs::Env::Staging;
    let key = env::var("MILVUE_API_KEY").unwrap();
    #[allow(unused_variables)]
    let dicom_list = vec![
        open_file("DX000000.dcm").unwrap(),
        open_file("DX000001.dcm").unwrap(),
    ];
    // let post_response = match milvue_rs::post(env, key, dicom_list).await {
    //     Ok(res) => res,
    //     Err(e) => panic!("Error: {}", e),
    // };
    // // println!("Response {:#?}", post_response);
    // match post_response.status() {
    //     reqwest::StatusCode::OK => println!("Success!"),
    //     status => println!("Expected status 200 got: {:#?}", status),
    // }
    let status_response = match milvue_rs::get_study_status(
        env,
        key,
        "1.2.276.0.7230010.3.1.2.514589184.1.1664350894.244479".to_string(),
    )
    .await
    {
        Ok(res) => res,
        Err(e) => panic!("Error: {}", e),
    };

    let status_body: milvue_rs::GetStatusResponse = status_response.json().await.unwrap();

    println!("Response {:#?}", status_body);
}
