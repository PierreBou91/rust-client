use milvue_rs::post_milvue;

#[tokio::main]
pub async fn main() {
    post_milvue().await.unwrap();
}
