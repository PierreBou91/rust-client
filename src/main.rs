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

    let study_instance_uid = dicom_list[0].element_by_name("StudyInstanceUID").unwrap();
    println!(
        "Study Instance UID: {}",
        study_instance_uid.to_str().unwrap().as_ref()
    );

    // let post_response = match milvue_rs::post(&env, &key, &dicom_list).await {
    //     Ok(res) => res,
    //     Err(e) => panic!("Error: {}", e),
    // };
    // // println!("Response {:#?}", post_response);
    // match post_response.status() {
    //     reqwest::StatusCode::OK => println!("Success!"),
    //     status => println!("Expected status 200 got: {:#?}", status),
    // }

    // let status_response = match milvue_rs::get_study_status(
    //     &env,
    //     &key,
    //     "1.2.276.0.7230010.3.1.2.514589184.1.1664350894.244479",
    // )
    // .await
    // {
    //     Ok(res) => res,
    //     Err(e) => panic!("Error: {}", e),
    // };

    // let status_body: milvue_rs::StatusResponse = status_response.json().await.unwrap();

    // println!("Response {:#?}", status_body);

    match milvue_rs::wait_for_done(&env, &key, study_instance_uid.to_str().unwrap().as_ref()).await
    {
        Ok(_) => println!("Done!"),
        Err(e) => panic!("Error: {}", e),
    }
    let res = milvue_rs::get::get(
        &env,
        &key,
        study_instance_uid.to_str().unwrap().as_ref(),
        &milvue_rs::MilvueParams::default(),
    )
    .await;

    println!("Response {:#?}", res);
}
