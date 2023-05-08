use serde::Deserialize;

pub enum Env {
    Dev,
    Staging,
    Prod,
}

#[derive(Deserialize, Debug)]
pub struct GetStatusResponse {
    #[serde(rename = "StudyInstanceUID")]
    pub study_instance_uid: String,
    pub status: String,
    pub version: String,
    pub message: String,
}
