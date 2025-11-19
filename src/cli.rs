use std::{path::PathBuf, pin::pin, process};

use clap::{Args, Parser, Subcommand};
use color_eyre::eyre::{Result, WrapErr as _};
use futures_util::StreamExt as _;
use tokio::fs;

use crate::Client;

fn compact<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap()
}

fn pretty<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap()
}

fn json_string<T: serde::Serialize>(value: &T, pretty_: bool) -> String {
    if pretty_ {
        pretty(value)
    } else {
        compact(value)
    }
}

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Gyazo {
    /// The `Gyazo_session` cookies, giving access to internal APIs
    #[clap(global = true, long, env = "GYAZO_COOKIE")]
    pub cookie: Option<String>,
    /// Identifier for the device, also known as "Gyzao ID"
    #[clap(global = true, long, short, env = "GYAZO_DEVICE")]
    pub device: Option<String>,
    /// API key, also known as "access token"
    #[clap(global = true, long, short, env = "GYAZO_KEY")]
    pub key: Option<String>,
    #[clap(global = true, long, conflicts_with = "cookie"/*, env = "GYAZO_NO_COOKIE"*/)]
    pub no_cookie: bool,
    #[clap(global = true, long, conflicts_with = "device")]
    pub no_device: bool,
    #[clap(global = true, long, conflicts_with = "key")]
    pub no_key: bool,
    #[clap(subcommand)]
    pub command: Command,
}

// Maybe:
// - Upload [--open] [ --capture | --record | <file> ] (needs ID or key)
// - Download (needs file ID, determines image vs video)
// - Api (needs key)
//   - Count
//   - Get
//   - List
//   - ...
//
// - Link:
//   - generate random PNG
//   - upload it
//   - have user follow the URL, login
//   - do OAuth with https://gyoza.recipes to get API key
//     - alt. explain how to do it
//   - call API delete to ensure everything worked
//   - steal a cookie too?
//
// Note that it seems possible to upload without an account,
// this creates a shadow account with a single linked device,
// that later can have an account created for.
// Unclear how to construct a URL to open a browser session for this account.
#[derive(Debug, Subcommand)]
pub enum Command {
    // Gui(crate::Gui),
    Image(Image),
    Video(Video),
    Count(Count),
    #[clap(aliases = ["down", "dl"])]
    Download(Download),
    #[clap(aliases = ["file", "info"])]
    Get(Get),
    #[clap(alias = "ls")]
    List(List),
    #[clap(alias = "up")]
    Upload(Upload),
    Config,
    #[clap(hide = true)]
    ConfigDir,
    #[clap(hide = true)]
    ConfigPath,
    Me(Me),
}

impl Gyazo {
    pub fn new() -> Self {
        <Gyazo as Parser>::parse()
    }

    pub async fn run(self) -> Result<()> {
        use crate::Config;
        use Command::*;

        let mut config = Config::load();
        config.cookie = if self.no_cookie {
            None
        } else {
            self.cookie.clone().or(config.cookie)
        };
        config.device = if self.no_device {
            None
        } else {
            self.device.clone().or(config.device)
        };
        config.key = if self.no_key {
            None
        } else {
            self.key.clone().or(config.key)
        };

        let client = &Client::new(&config);

        match self.command {
            // Gui(cmd) => cmd.run().unwrap(),
            Image(cmd) => cmd.run(client).await?,
            Count(cmd) => cmd.run(client).await?,
            Download(cmd) => cmd.run(client).await?,
            Get(cmd) => cmd.run(client).await?,
            List(cmd) => cmd.run(client).await?,
            Video(cmd) => cmd.run(client).await?,
            Upload(cmd) => cmd.run(client).await?,
            Config => {
                let config = toml::to_string_pretty(&config)
                    .wrap_err("Failed to serialize config file as TOML")?;
                print!("{config}");
            }
            ConfigDir => println!("{}", Config::dir().display()),
            ConfigPath => println!("{}", Config::path().display()),
            Me(cmd) => cmd.run(client).await?,
        }
        Ok(())
    }
}

#[derive(Args, Clone, Debug)]
pub struct UploadArgs {
    #[clap(action, long, short, alias = "anon")]
    pub anonymous: bool,
    #[clap(long)]
    pub app: Option<String>,
    // #[clap(action, long)]
    // pub public: bool,
    // #[clap(action, conflicts_with = "public", long)]
    // pub private: bool,
    #[clap(action, long, alias = "private-meta")]
    pub public_metadata: bool,
    #[clap(
        action,
        conflicts_with = "public_metadata",
        long,
        alias = "public-meta"
    )]
    pub private_metadata: bool,
}

#[derive(Args, Clone, Debug)]
pub struct Open {
    #[clap(action, long)]
    pub open: bool,
}

impl UploadArgs {
    pub fn update(&self, mut upload: crate::Upload) -> crate::Upload {
        if let Some(app) = self.app.as_ref() {
            upload.app = app.clone();
        }
        // if self.public {
        //     upload.public_access = true;
        // }
        // if self.private {
        //     upload.public_access = false;
        // }
        if self.public_metadata {
            upload.public_metadata = true;
        }
        if self.private_metadata {
            upload.public_metadata = false;
        }
        if self.anonymous {
            upload.anonymous = true;
        }
        upload
    }
}

#[derive(Args, Debug)]
pub struct Image {
    #[clap(flatten)]
    upload: UploadArgs,
    #[clap(flatten)]
    open: Open,
}

impl Image {
    pub async fn run(self, client: &Client) -> Result<()> {
        let file = tempfile::NamedTempFile::with_suffix(".png")
            .wrap_err("Failed to create a temporary PNG file")?;
        let path = file.path().to_str().unwrap();
        println!("Select the region to capture");
        let _ = process::Command::new("import")
            .args(&[path])
            .output()
            .wrap_err("Failed to run ImageMagick `import` command to capture an image")?;

        let size = file
            .path()
            .metadata()
            .wrap_err("Failed to determine size of captured image")?
            .len();
        println!("Uploading {size} bytes");
        let upload = self.upload.update(crate::Upload::default());
        let url = client
            .upload_image(file.path(), &upload)
            .await
            .wrap_err("Failed to upload captured image")?;
        println!("URL: {url}");
        if self.open.open {
            open::that(url.as_str()).wrap_err("Failed to open URL in browser")?;
        }
        Ok(())
    }
}

struct VideoName;

impl VideoName {
    fn path() -> PathBuf {
        let mut path = PathBuf::new();
        path.push("video.mp4");
        path
    }
}

use clap::builder::{IntoResettable, OsStr, Resettable};

impl IntoResettable<OsStr> for VideoName {
    fn into_resettable(self) -> Resettable<OsStr> {
        Resettable::Value(Self::path().into_os_string().into())
    }
}

#[derive(Args, Debug)]
pub struct Video {
    #[clap(long, short, alias = "time", short_alias = 't', default_value_t = 7)]
    pub seconds: u64,
    #[clap(flatten)]
    open: Open,
    #[clap(long, default_missing_value = VideoName, num_args=0..=1)]
    save: Option<PathBuf>,
}

impl Video {
    pub async fn run(self, client: &Client) -> Result<()> {
        // select
        println!("Select the region to record");
        let output = process::Command::new("slop")
            .args(&["-f", ":0.0+%x,%y %wx%h"])
            .output()
            .wrap_err("Failed to select rectangle to record video within")?;
        let mut it = std::str::from_utf8(&output.stdout)
            .wrap_err("`slop` output was not UTF8")?
            .split(' ');
        let xy = it.next().unwrap();
        let wh = it.next().unwrap();

        // the encoder needs height+width divisible by two
        println!("wh before: {wh}");
        let (w, h) = wh.split_once('x').unwrap();
        let w = (w.parse::<u32>().unwrap() / 2) * 2;
        let h = (h.parse::<u32>().unwrap() / 2) * 2;
        let wh = format!("{w}x{h}");
        println!("wh after: {wh}");

        // record
        let file = tempfile::NamedTempFile::with_suffix(".mp4")
            .wrap_err("Failed to create a temporary MP4 file")?;
        let path = file.path().to_str().unwrap();
        println!("path: {path}");
        let seconds = self.seconds.to_string();
        let args = &[
            "-f",
            "x11grab",
            "-s",
            wh.as_str(),
            "-i",
            xy,
            "-f",
            "alsa",
            "-i",
            "pulse",
            "-t",
            seconds.as_str(),
            // without this, videos won't play on Windows
            "-pix_fmt",
            "yuv420p",
            "-y",
            // uhh.. Stream #0: not enough frames to estimate rate; consider increasing probesize
            "-framerate",
            "24",
            "-probesize",
            "64MB",
            path,
        ];
        println!("ffmpeg: {}", args.join(" "));
        println!("Recording for {seconds} seconds");
        process::Command::new("ffmpeg")
            .args(args)
            .output()
            .wrap_err("Failed to run FFmpeg to record a video")?;
        let size = file
            .path()
            .metadata()
            .wrap_err("Failed to determine size of recorded video")?
            .len();

        if let Some(path) = self.save.as_ref() {
            // use md5::Digest as _;
            // let bytes = fs::read(file.path()).await.unwrap();
            // let digest = hex::encode(md5::Md5::digest(&bytes));
            // println!("hex: {digest}");
            println!("Saving to {}", path.display());
            fs::copy(file.path(), path).await.unwrap();
        }

        // upload
        println!("Uploading {size} bytes");
        let url = client
            .upload_video(file.path())
            .await
            .wrap_err("Failed to upload recorded video")?;
        println!("URL: {url}");
        if self.open.open {
            open::that(url.as_str()).wrap_err("Failed to open URL in browser")?;
        }
        Ok(())
    }
}

#[derive(Args, Debug)]
pub struct Count;

impl Count {
    pub async fn run(self, client: &Client) -> Result<()> {
        let count = client
            .count()
            .await
            .wrap_err("Failed to determine number of stored files")?;
        println!("{count}");
        Ok(())
    }
}

#[derive(Args, Debug)]
pub struct Me;

impl Me {
    pub async fn run(self, client: &Client) -> Result<()> {
        let me = client
            .me()
            .await
            .wrap_err("Failed to determine user information")?;
        println!("{}", pretty(&me));
        Ok(())
    }
}

#[derive(Args, Debug)]
pub struct Get {
    id: String,
}

impl Get {
    pub async fn run(self, client: &Client) -> Result<()> {
        let file = client
            .get(&self.id)
            .await
            .wrap_err("Failed to determine file information")?;
        println!("{}", pretty(&file));
        Ok(())
    }
}

#[derive(Args, Debug)]
pub struct List {
    #[clap(action, long)]
    pub internal: bool,
    #[clap(action, long)]
    pub pretty: bool,
}

impl List {
    pub async fn run(self, client: &Client) -> Result<()> {
        if self.internal {
            let mut files = pin!(client.list_internal().await);
            while let Some(file) = files.next().await {
                let file =
                    file.wrap_err("Failed to determine file information with internal API")?;
                println!("{}", json_string(&file, self.pretty));
            }
        } else {
            let mut files = pin!(client.list().await);
            while let Some(file) = files.next().await {
                let file = file.wrap_err("Failed to determine file information with API")?;
                println!("{}", json_string(&file, self.pretty));
            }
        }
        Ok(())
    }
}

#[derive(Args, Debug)]
pub struct Download {
    #[clap(long)]
    pub to: Option<PathBuf>,
    #[clap(action, long, short)]
    pub force: bool,
    pub id: String,
}

impl Download {
    pub async fn run(self, client: &Client) -> Result<()> {
        use futures_util::StreamExt as _;

        let info = client
            .get(&self.id)
            .await
            .wrap_err("Failed to determine file information")?;
        let mut byte_stream = reqwest::get(info.download.as_str())
            .await
            .wrap_err("Failed to connecto to file download URL")?
            .bytes_stream();

        let path = self.to.unwrap_or_else(|| info.name().into());
        let path_str = path.display().to_string();
        println!("File: {path_str}");
        let mut file = if self.force {
            fs::File::create(path)
                .await
                .wrap_err_with(|| format!("Failed to create file {path_str}"))?
        } else {
            fs::File::create_new(path)
                .await
                .wrap_err_with(|| format!("Failed to create new file {path_str}"))?
        };
        while let Some(bytes) = byte_stream.next().await {
            tokio::io::copy(
                &mut bytes
                    .wrap_err("Failed to read bytes from file download URL")?
                    .as_ref(),
                &mut file,
            )
            .await
            .wrap_err("Failed to copy bytes to destination")?;
        }
        file.sync_all()
            .await
            .wrap_err("Failed to flush file to disk")?;
        let size = file
            .metadata()
            .await
            .wrap_err("Failed to determine size of downloaded file")?
            .len();
        println!("Size: {size} bytes");
        Ok(())
    }
}

#[derive(Args, Debug)]
pub struct Upload {
    #[clap(flatten)]
    upload: UploadArgs,
    pub file: PathBuf,
}

impl Upload {
    pub async fn run(self, client: &Client) -> Result<()> {
        let file = &self.file;
        let is_mp4 = Some("mp4".as_ref()) == file.extension();
        let upload = self.upload.update(crate::Upload::default());
        let file_str = file.display().to_string();
        if is_mp4 {
            let url = client
                .upload_video(file)
                .await
                .wrap_err_with(|| format!("Failed to upload video file {file_str}"))?;
            println!("URL: {url}");
        } else {
            let (url, device) = client
                .upload_image_cgi(file, &upload)
                .await
                .wrap_err_with(|| format!("Failed to upload image file {file_str}"))?;
            println!("Device: {device}");
            println!("URL: {url}");
        }
        Ok(())
    }
}
