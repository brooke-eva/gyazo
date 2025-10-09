use std::path::Path;

use futures_util::Stream;
use http::HeaderMap;
#[macro_use(Deserialize, Serialize)]
extern crate serde;
use thiserror::Error;

#[cfg(feature = "cli")]
pub mod cli;
mod config;
pub use config::Config;
mod image;
pub use image::Image;

#[derive(Clone)]
pub struct Client {
    access_token: Option<String>,
}

pub const API_URL: &str = "https://api.gyazo.com/api";
pub const IMAGE_UPLOAD_URL: &str = "https://upload.gyazo.com/api/upload";
pub const VIDEO_UPLOAD_URL: &str = "https://gif.gyazo.com/gif/upload";

pub const DEFAULT_APP: &str = "https://github.com/brooke-eva/gyazo";

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug, Deserialize, Error, Serialize)]
pub enum Error {
    #[error("error")]
    String(String),
}

pub type GyazoId = String;

// eg. "2018-07-24T07:33:24.771Z"
pub type Timestamp = String;

impl Client {
    pub fn new(access_token: Option<&str>) -> Self {
        let access_token = access_token
            .map(str::to_string)
            // lookup in config file if access_token is None
            .or_else(|| Config::load().access_token);
        Self { access_token }
    }

    async fn get_url<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        query: &[(&str, &str)],
    ) -> Result<(T, HeaderMap)> {
        let response = reqwest::Client::new()
            .get(url)
            .query(&[("access_token", self.access_token.as_ref().unwrap())])
            .query(query)
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        let headers = response.headers().clone();
        Ok((response.json().await.unwrap(), headers))
    }

    pub async fn get(&self, image_id: &str) -> Result<Image> {
        let url = &format!("{API_URL}/images/{image_id}");
        let query = &[];

        self.get_url(url, query).await.map(|(image, _)| image)
    }

    pub async fn count(&self) -> Result<usize> {
        let url = &format!("{API_URL}/images");
        let query = &[("per_page", "0")];

        let (_, headers): (Vec<Image>, _) = self.get_url(url, query).await?;
        let count: usize = headers
            .get("x-total-count")
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        Ok(count)
    }

    pub async fn list(&self) -> impl Stream<Item = Result<Image>> {
        async_stream::try_stream! {
            let mut page_number = 1;
            let mut received = 0;
            loop {
                let url = &format!("{API_URL}/images");
                let page_number_string = page_number.to_string();
                let query = &[("page", page_number_string.as_str()), ("per_page", "100")];
                let (page, headers): (Vec<Image>, _) = self.get_url(url, query).await?;
                received += page.len();

                for image in page.into_iter() {
                    yield image;
                }

                let count: usize = headers.get("x-total-count").unwrap().to_str().unwrap().parse().unwrap();
                if received >= count {
                    break;
                }
                page_number += 1;
            }
        }
    }

    pub async fn list_vec(&self) -> Result<Vec<Image>> {
        use futures_util::TryStreamExt as _;

        self.list().await.try_collect().await
    }

    // allowed types: jpg, png, gif
    // mp4: pro/teams user only
    pub async fn upload_image(
        &self,
        path: &Path,
        upload: &Upload,
    ) -> Result<(Image, Option<GyazoId>)> {
        // let access_policy = if upload.public_access { "anyone" } else { "only_me" };
        let metadata_is_public = upload.public_metadata.to_string();

        let query = &[
            ("app", upload.app.as_str()),
            // ("access_policy", access_policy),
            ("metadata_is_public", metadata_is_public.as_str()),
        ];

        let form = reqwest::multipart::Form::new()
            .file("imagedata", path)
            .await
            .unwrap();

        let mut request = reqwest::Client::new()
            .post(IMAGE_UPLOAD_URL)
            .multipart(form)
            .query(&[("access_token", self.access_token.as_ref().unwrap())])
            .query(&query);

        if let Some(created) = path.metadata().ok().and_then(|meta| meta.created().ok()) {
            let created_at = created
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs_f32();
            // This shows as "Uploaded at"
            request = request.query(&[("created_at", created_at)]);
        };

        let response = request.send().await.unwrap();
        assert!(response.status().is_success());

        let gyazo_id = response
            .headers()
            .get("x-gyazo-id")
            .map(|gyazo_id| gyazo_id.to_str().unwrap().to_string());
        Ok((response.json().await.unwrap(), gyazo_id))
    }

    pub async fn upload_video(&self, path: &Path) -> Result<String> {
        let form = reqwest::multipart::Form::new()
            .file("data", path)
            .await
            .unwrap();

        let request = reqwest::Client::new()
            .post(VIDEO_UPLOAD_URL)
            .multipart(form)
            .query(&[("access_token", self.access_token.as_ref().unwrap())]);

        let response = request.send().await.unwrap();
        // println!("{response:?}");
        let permalink_url = response.text().await.unwrap();
        Ok(permalink_url)
    }
}

pub struct Upload {
    pub app: String,
    // pub public_access: bool,
    pub public_metadata: bool,
}

impl Upload {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for Upload {
    fn default() -> Self {
        let config = Config::load();
        Self {
            app: DEFAULT_APP.to_string(),
            // public_access: config.upload.public_access,
            public_metadata: config.upload.public_metadata,
        }
    }
}
