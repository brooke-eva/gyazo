use crate::Timestamp;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Image {
    pub image_id: String,
    pub permalink_url: String,
    pub thumb_url: Option<String>,
    #[serde(rename = "type")]
    pub file_type: String,
    pub created_at: Timestamp,
    pub metadata: Option<Metadata>,
    pub ocr: Option<Ocr>,
}

impl Image {
    pub fn download_url(&self) -> String {
        if &self.file_type == "mp4" {
            format!(
                "https://i.gyazo.com/download/{}.{}",
                self.image_id, self.file_type
            )
        } else {
            format!("https://i.gyazo.com/{}.{}", self.image_id, self.file_type)
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Metadata {
    pub app: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub desc: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ocr {
    pub locale: String,
    pub description: String,
}
