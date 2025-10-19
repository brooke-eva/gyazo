//! # Better Gyazo for Linux
//!
//! Note that the URLs are always set to the 16 byte MD5 hash
//! of the image or video.

use std::{io, path::Path};

use futures_util::Stream;
use http::HeaderMap;
pub use http::StatusCode;
#[macro_use(Deserialize, Serialize)]
extern crate serde;
use serde_json::{Value, json};
use thiserror::Error;
pub use url::Url;

#[cfg(feature = "cli")]
pub mod cli;
mod config;
pub use config::Config;
mod image;
pub use image::{File, Image};

// pub struct Auth<'a> {
//     pub id: Option<&'a str>,
//     pub key: Option<&'a str>,
// }

#[derive(Clone)]
pub struct Client {
    cookie: Option<String>,
    device: Option<String>,
    key: Option<String>,
}

pub const API_URL: &str = "https://api.gyazo.com/api";
pub const API_IMAGE_UPLOAD_URL: &str = "https://upload.gyazo.com/api/upload";
pub const CGI_IMAGE_UPLOAD_URL: &str = "https://upload.gyazo.com/upload.cgi";
pub const VIDEO_UPLOAD_URL: &str = "https://gif.gyazo.com/gif/upload";

// Maybe "Uploaded with Gyoza: <url>"?
// And can override this (to use some detected app instead of an ad)
// in the config.
pub const DEFAULT_APP: &str = "https://github.com/brooke-eva/gyazo";

pub type Result<T, E = Error> = core::result::Result<T, E>;

// #[derive(Debug, Deserialize, Error, Serialize)]
#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    String(String),
    #[error("{message}")]
    Missing { message: String },
    #[error("{message}")]
    Request {
        message: String,
        source: reqwest::Error,
    },
    #[error("{message}")]
    Io { message: String, source: io::Error },
    #[error("{message}")]
    Url {
        message: String,
        source: url::ParseError,
    },
    #[error("{message} ({text})")]
    Api {
        message: String,
        status: ApiStatus,
        text: String,
    },
    #[error("{message} - type {type_name} ({text})")]
    Json {
        message: String,
        text: String,
        source: serde_json::Error,
        type_name: &'static str,
    },
}

#[derive(Debug, Error)]
pub enum ApiStatus {
    #[error("invalid request")]
    InvalidRequest,
    #[error("unauthenticated")]
    Unauthenticated,
    #[error("Pro required")]
    ProRequired,
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("unprocessable content")]
    Unprocessable,
    #[error("rate limited")]
    RateLimited,
    #[error("unexpected")]
    Unexpected,
    #[error("undocumented status code: {0}")]
    Undocumented(StatusCode),
}

pub trait ExtractJson<T>: Sized {
    #[allow(async_fn_in_trait)]
    async fn extract_json<D>(self, msg: D) -> Result<T>
    where
        D: core::fmt::Display + Send + Sync + 'static;
}

impl<T: serde::de::DeserializeOwned + TypeName> ExtractJson<T> for reqwest::Response {
    async fn extract_json<D>(self, msg: D) -> Result<T>
    where
        D: core::fmt::Display + Send + Sync + 'static,
    {
        let text = self
            .text()
            .await
            .wrap_err("response contained invalid text")?;
        match serde_json::from_str(&text) {
            Ok(t) => Ok(t),
            Err(source) => Err(Error::Json {
                message: msg.to_string(),
                text,
                source,
                type_name: T::type_name(),
            }),
        }
    }
}

pub trait Verify: Sized {
    #[allow(async_fn_in_trait)]
    async fn verify<D>(self, msg: D) -> Result<Self>
    where
        D: core::fmt::Display + Send + Sync + 'static;
}

impl Verify for reqwest::Response {
    async fn verify<D>(self, msg: D) -> Result<Self>
    where
        D: core::fmt::Display + Send + Sync + 'static,
    {
        use ApiStatus::*;

        let status = self.status();
        if status.is_success() {
            return Ok(self);
        }
        let status = match status.as_u16() {
            400 => InvalidRequest,
            401 => Unauthenticated,
            402 => ProRequired,
            403 => Unauthorized,
            404 => NotFound,
            422 => Unprocessable,
            429 => RateLimited,
            500 => Unexpected,
            _ => Undocumented(status),
        };
        let text = self
            .text()
            .await
            .unwrap_or_else(|_| "TEXT MISSING".to_string());
        // println!("{text}");
        Err(Error::Api {
            message: msg.to_string(),
            status,
            text,
        })
    }
}

pub trait WrapNone<T> {
    fn wrap_none<D>(self, msg: D) -> Result<T>
    where
        D: core::fmt::Display + Send + Sync + 'static;
}

impl<T> WrapNone<T> for Option<T> {
    fn wrap_none<D>(self, msg: D) -> Result<T>
    where
        D: core::fmt::Display + Send + Sync + 'static,
    {
        match self {
            Some(t) => Ok(t),
            None => Err(Error::Missing {
                message: msg.to_string(),
            }),
        }
    }
}

pub trait WrapErr<T, E> {
    fn wrap_err<D>(self, msg: D) -> Result<T>
    where
        D: core::fmt::Display + Send + Sync + 'static;
}

impl<T> WrapErr<T, io::Error> for io::Result<T> {
    fn wrap_err<D>(self, msg: D) -> Result<T>
    where
        D: core::fmt::Display + Send + Sync + 'static,
    {
        match self {
            Ok(t) => Ok(t),
            Err(source) => Err(Error::Io {
                message: msg.to_string(),
                source,
            }),
        }
    }
}

impl<T> WrapErr<T, reqwest::Error> for reqwest::Result<T> {
    fn wrap_err<D>(self, msg: D) -> Result<T>
    where
        D: core::fmt::Display + Send + Sync + 'static,
    {
        match self {
            Ok(t) => Ok(t),
            Err(source) => Err(Error::Request {
                message: msg.to_string(),
                source,
            }),
        }
    }
}

impl WrapErr<Url, url::ParseError> for core::result::Result<Url, url::ParseError> {
    fn wrap_err<D>(self, msg: D) -> Result<Url>
    where
        D: core::fmt::Display + Send + Sync + 'static,
    {
        match self {
            Ok(t) => Ok(t),
            Err(source) => Err(Error::Url {
                message: msg.to_string(),
                source,
            }),
        }
    }
}

pub type Device = String;

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub email: String,
    pub is_pro: bool,
    pub is_team: bool,
    // prefix of email before @?
    pub name: String,
    // "//assets2.gyazo.com/assets/images/common/default_user_icon-xxx.svg"
    pub profile_image: String,
    // 12 bytes
    pub uid: String,
}

// eg. "2018-07-24T07:33:24.771Z"
pub type Timestamp = String;

pub trait TypeName {
    fn type_name() -> &'static str;
}

impl TypeName for Url {
    fn type_name() -> &'static str {
        "URL"
    }
}

impl TypeName for Value {
    fn type_name() -> &'static str {
        "value"
    }
}

impl TypeName for Vec<Value> {
    fn type_name() -> &'static str {
        "vector of values"
    }
}

impl TypeName for Image {
    fn type_name() -> &'static str {
        "Image"
    }
}

impl TypeName for Vec<Image> {
    fn type_name() -> &'static str {
        "vector of Image"
    }
}

impl Client {
    pub fn new(config: &Config) -> Self {
        Self {
            cookie: config.cookie.clone(),
            device: config.device.clone(),
            key: config.key.clone(),
        }
    }

    pub fn expect_cookie(&self) -> Result<&str> {
        self.cookie.as_deref().wrap_none("No cookie configured")
    }

    pub fn expect_device(&self) -> Result<&str> {
        self.device.as_deref().wrap_none("No device ID configured")
    }

    pub fn expect_key(&self) -> Result<&str> {
        self.key.as_deref().wrap_none("No API key configured")
    }

    async fn api_get_with_headers<T>(
        &self,
        url: &str,
        query: &[(&str, &str)],
    ) -> Result<(T, HeaderMap)>
    where
        T: serde::de::DeserializeOwned + TypeName,
    {
        let response = reqwest::Client::new()
            .get(url)
            .query(&[("access_token", self.expect_key()?)])
            .query(query)
            .send()
            .await
            .wrap_err("Could not send API get request")?
            .verify(format!("API get request to `{url}` failed"))
            .await?;

        let headers = response.headers().clone();

        response
            .extract_json("Could not decode API get request response as JSON")
            .await
            .map(|t| (t, headers))
    }

    async fn api_get<T>(&self, url: &str, query: &[(&str, &str)]) -> Result<T>
    where
        T: serde::de::DeserializeOwned + TypeName,
    {
        self.api_get_with_headers(url, query).await.map(|(t, _)| t)
    }

    async fn internal_api_get<T>(&self, url: &str, query: &[(&str, &str)]) -> Result<T>
    where
        T: serde::de::DeserializeOwned + TypeName,
    {
        reqwest::Client::new()
            .get(url)
            .header("cookie", format!("Gyazo_session={}", self.expect_cookie()?))
            .query(query)
            .send()
            .await
            .wrap_err("Could not send internal API get request")?
            .verify(format!("Internal API get request to `{url}` failed"))
            .await?
            .extract_json("Could not decode internal API get request response as JSON")
            .await
    }

    pub async fn get(&self, image_id: &str) -> Result<File> {
        let url = &format!("{API_URL}/images/{image_id}");

        Ok(self.api_get::<Image>(url, &[]).await?.into_file().await)
    }

    pub async fn count(&self) -> Result<usize> {
        let url = &format!("{API_URL}/images");
        let query = &[("per_page", "0")];

        let (_, headers): (Vec<Image>, _) = self.api_get_with_headers(url, query).await?;
        let Some(Ok(Ok(count))) = headers
            .get("x-total-count")
            .map(|header| header.to_str().map(str::parse::<usize>))
        else {
            return Err(Error::String("API did not respond to an empty read query with a parseable `X-Total-Count` header".to_string()));
        };
        Ok(count)
    }

    pub async fn me(&self) -> Result<User> {
        let url = &format!("{API_URL}/users/me");

        #[derive(Deserialize)]
        struct WrappedUser {
            user: User,
        }

        impl TypeName for WrappedUser {
            fn type_name() -> &'static str {
                "user wrapped in object"
            }
        }

        let wrapped_me: WrappedUser = self.api_get(url, &[]).await?;
        Ok(wrapped_me.user)
    }

    // To get a Result<Vec<File>>, use futures::TryStreamExt::try_collect
    pub async fn list(&self) -> impl Stream<Item = Result<File>> {
        async_stream::try_stream! {
            let mut page_number = 1;
            let mut received = 0;
            loop {
                let url = &format!("{API_URL}/images");
                let page_number_string = page_number.to_string();
                let query = &[("page", page_number_string.as_str()), ("per_page", "100")];
                let (page, headers): (Vec<Image>, _) = self.api_get_with_headers(url, query).await?;
                received += page.len();

                for mut image in page.into_iter() {
                    image.fix_mp4().await;
                    yield image.into_file().await;
                }

                let count: usize = headers.get("x-total-count").unwrap().to_str().unwrap().parse().unwrap();
                if received >= count {
                    break;
                }
                page_number += 1;
            }
        }
    }

    pub async fn list_internal(&self) -> impl Stream<Item = Result<Value>> {
        async_stream::try_stream! {
            let mut page_number = 1;
            loop {
                let url = &format!("{API_URL}/internal/images");
                let page_number_string = page_number.to_string();
                let query = &[("page", page_number_string.as_str()), ("per_page", "100")];
                let page: Vec<Value> = self.internal_api_get(url, query).await?;
                if page.is_empty() {
                    break;
                }

                for image in page.into_iter() {
                    yield image;
                }

                page_number += 1;
            }
        }
    }

    pub async fn upload_image(&self, path: &Path, upload: &Upload) -> Result<Url> {
        // if self.id.is_some() {
        self.upload_image_cgi(path, upload)
            .await
            .map(|(url, _)| url)
        // } else if self.key.is_some() {
        //     let (image, _) = self.upload_image_api(path, upload).await?;
        //     Ok(image.permalink_url)
        // } else {
        //     panic!("Need access token or device ID");
        // }
    }

    // allowed types: jpg, png, gif
    // mp4: pro/teams user only
    pub async fn upload_image_cgi(&self, path: &Path, upload: &Upload) -> Result<(Url, Device)> {
        let device = if upload.anonymous {
            None
        } else {
            self.device.clone()
        };
        let form = reqwest::multipart::Form::new()
            .text("id", device.clone().unwrap_or_default())
            .text(
                "metadata",
                json!({
                    "app": upload.app.as_str(),
                    // ends up in "Source"
                    // "title": ...,
                    // no obvious effect
                    // "url": ...,
                    // no obvious effect
                    // "note": ...,
                })
                .to_string(),
            )
            .file("imagedata", path)
            .await
            .wrap_err("Could not prepare image upload form")?;

        let response = reqwest::Client::new()
            .post(CGI_IMAGE_UPLOAD_URL)
            .multipart(form)
            // .header("User-Agent", "Gyazo/1.3.2")
            // returns a session token in x-gyazo-session-token
            .header("x-gyazo-accept-token", "required")
            .send()
            .await
            .wrap_err("Could not send CGI image upload request")?
            .verify("CGI image upload request failed")
            .await?;

        let headers = response.headers();
        let maybe_token = headers
            .get("x-gyazo-session-token")
            .map(|token| token.to_str().unwrap().to_string());
        // If we didn't send device ID... expect to receive one

        let device = device.unwrap_or_else(|| {
            headers
                .get("x-gyazo-id")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        });

        let mut url = response
            .text()
            .await
            .wrap_err("CGI image upload response did not contain text")?
            .parse::<Url>()
            .wrap_err("CGI image upload response did not contain a URL")?;

        if let Some(token) = maybe_token {
            url.set_query(Some(&format!("token={token}")));
        }
        Ok((url, device))
    }

    // allowed types: jpg, png, gif
    // mp4: pro/teams user only
    pub async fn upload_image_api(&self, path: &Path, upload: &Upload) -> Result<File> {
        // let access_policy = if upload.public_access { "anyone" } else { "only_me" };
        let public_metadata = upload.public_metadata.to_string();

        let query = &[
            ("app", upload.app.as_str()),
            // ("access_policy", access_policy),
            ("metadata_is_public", public_metadata.as_str()),
        ];

        let form = reqwest::multipart::Form::new()
            .file("imagedata", path)
            .await
            .wrap_err("Could not prepare image upload form")?;

        let mut request = reqwest::Client::new()
            .post(API_IMAGE_UPLOAD_URL)
            .multipart(form)
            // .header("User-Agent", "Gyazo/1.3.2")
            // .header("x-gyazo-accept-token", "required")
            .query(&[("access_token", self.expect_key()?)])
            .query(&query);

        if let Some(created) = path.metadata().ok().and_then(|meta| meta.created().ok()) {
            let created_at = created
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs_f32();
            // This shows as "Uploaded at"
            request = request.query(&[("created_at", created_at)]);
        };

        let image: Image = request
            .send()
            .await
            .wrap_err("Could not send API image upload request")?
            .verify("API image upload failed")
            .await?
            .extract_json("Could not decode image API upload response as JSON")
            .await?;

        Ok(image.into_file().await)
    }

    pub async fn upload_video(&self, path: &Path) -> Result<Url> {
        let form = reqwest::multipart::Form::new()
            .text("id", self.expect_device()?.to_string())
            .file("data", path)
            .await
            .wrap_err("Could not prepare video upload form")?;

        let url = reqwest::Client::new()
            .post(VIDEO_UPLOAD_URL)
            .multipart(form)
            .send()
            .await
            .wrap_err("Could not send video upload request")?
            .verify("Video upload failed")
            .await?
            .text()
            .await
            .wrap_err("Video API upload response did not contain text")?;

        Url::parse(&url).wrap_err("Video API upload response did not contain a URL")
    }
}

pub struct Upload {
    pub app: String,
    // pub public_access: bool,
    pub public_metadata: bool,
    pub anonymous: bool,
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
            anonymous: false,
        }
    }
}
