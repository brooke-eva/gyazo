use crate::{Timestamp, Url};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Image {
    pub image_id: String,
    pub permalink_url: Url,
    pub thumb_url: Option<Url>,
    #[serde(rename = "type")]
    pub file_type: String,
    pub created_at: Timestamp,
    pub metadata: Option<Metadata>,
    pub ocr: Option<Ocr>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Metadata {
    #[serde(default, skip_serializing_if = "useless")]
    pub app: Option<String>,
    #[serde(default, skip_serializing_if = "useless")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "useless")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub desc: String,
}

fn useless(value: &Option<String>) -> bool {
    value.as_deref().map(str::is_empty).unwrap_or(true)
}

fn empty(meta: &Option<Metadata>) -> bool {
    meta.as_ref()
        .map(|meta| {
            useless(&meta.app) && useless(&meta.title) && useless(&meta.url) && meta.desc.is_empty()
        })
        .unwrap_or(true)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct File {
    pub id: String,
    #[serde(rename = "type")]
    pub file_type: String,
    pub create_time: Timestamp,
    pub download: Url,
    pub permalink: Url,
    // pub thumb: Option<Url>,
    #[serde(default, skip_serializing_if = "empty")]
    pub meta: Option<Metadata>,
}

impl File {
    pub fn name(&self) -> String {
        format!("{}.{}", self.id, self.file_type)
    }
}

impl Image {
    pub async fn into_file(mut self) -> File {
        self.fix_mp4().await;
        let download = Url::parse(&self.download_url()).unwrap();
        let Image {
            image_id: id,
            permalink_url: permalink,
            thumb_url: _thumb,
            file_type,
            created_at: create_time,
            metadata: meta,
            ..
        } = self;
        File {
            id,
            permalink,
            download,
            // thumb: _thumb,
            file_type,
            create_time,
            meta,
        }
    }
}

impl Image {
    fn mp4_download_url(&self) -> String {
        format!("https://i.gyazo.com/download/{}.mp4", self.image_id)
    }

    // The public API does not expose whether a "gif"
    // is actually an "mp4" or not. The HEAD does not
    // reveal if the MP4 download URL exists, so we
    // GET it without consuming the body.
    pub async fn fix_mp4(&mut self) {
        if self.file_type == "gif"
            && reqwest::get(self.mp4_download_url())
                .await
                .unwrap()
                .status()
                .is_success()
        {
            self.file_type = "mp4".to_string();
        }
    }

    pub fn download_url(&self) -> String {
        if &self.file_type == "mp4" {
            self.mp4_download_url()
        } else {
            format!("https://i.gyazo.com/{}.{}", self.image_id, self.file_type)
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ocr {
    pub locale: String,
    pub description: String,
}
