use std::{path::PathBuf, pin::pin, process};

use clap::{Args, Parser, Subcommand};
use futures_util::StreamExt as _;

use crate::{Client, Result};

fn pretty<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap()
}

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Gyazo {
    #[clap(global = true, long, short)]
    pub key: Option<String>,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Capture(Capture),
    Record(Record),
    Count(Count),
    #[clap(alias = "down")]
    Download(Download),
    Get(Get),
    List(List),
    #[clap(alias = "up")]
    Upload(Upload),
}

impl Gyazo {
    pub fn new() -> Self {
        <Gyazo as Parser>::parse()
    }

    pub async fn run(self) -> Result<()> {
        use Command::*;

        let client = &Client::new(self.key.as_deref());

        match self.command {
            Capture(cmd) => cmd.run(client).await,
            Count(cmd) => cmd.run(client).await,
            Download(cmd) => cmd.run().await,
            Get(cmd) => cmd.run(client).await,
            List(cmd) => cmd.run(client).await,
            Record(cmd) => cmd.run(client).await,
            Upload(cmd) => cmd.run(client).await,
        }
    }
}

#[derive(Args, Clone, Debug)]
pub struct UploadArgs {
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
        upload
    }
}

#[derive(Args, Debug)]
pub struct Capture {
    #[clap(flatten)]
    upload: UploadArgs,
    #[clap(flatten)]
    open: Open,
}

impl Capture {
    pub async fn run(self, client: &Client) -> Result<()> {
        let file = tempfile::NamedTempFile::with_suffix(".png").unwrap();
        let path = file.path().to_str().unwrap();
        println!("Select the region to capture");
        let _ = process::Command::new("import")
            .args(&[path])
            .output()
            .unwrap();

        let size = file.path().metadata().unwrap().len();
        println!("Uploading {size} bytes");
        let upload = self.upload.update(crate::Upload::default());
        let (image, _) = client.upload_image(file.path(), &upload).await?;
        println!("URL: {}", image.permalink_url);
        // println!("{image:?}");
        if self.open.open {
            open::that(image.permalink_url).ok();
        }
        Ok(())
    }
}

#[derive(Args, Debug)]
pub struct Record {
    #[clap(long, short, alias = "time", short_alias = 't', default_value_t = 7)]
    pub seconds: u64,
    #[clap(flatten)]
    open: Open,
}

impl Record {
    pub async fn run(self, client: &Client) -> Result<()> {
        // select
        println!("Select the region to record");
        let output = process::Command::new("slop")
            .args(&["-f", ":0.0+%x,%y %wx%h"])
            .output()
            .unwrap();
        let mut it = std::str::from_utf8(&output.stdout).unwrap().split(' ');
        let xy = it.next().unwrap();
        let wh = it.next().unwrap();

        // record
        let file = tempfile::NamedTempFile::with_suffix(".mp4").unwrap();
        let path = file.path().to_str().unwrap();
        let seconds = self.seconds.to_string();
        let args = &[
            "-f",
            "x11grab",
            "-s",
            wh,
            "-i",
            xy,
            "-f",
            "alsa",
            "-i",
            "pulse",
            "-t",
            seconds.as_str(),
            "-y",
            path,
        ];
        println!("Recording for {seconds} seconds");
        let _output = process::Command::new("ffmpeg").args(args).output().unwrap();
        // println!("{output:?}");
        let size = file.path().metadata().unwrap().len();
        println!("Uploading {size} bytes");
        let permalink = client.upload_video(file.path()).await?;
        println!("URL: {permalink}");
        if self.open.open {
            open::that(permalink).ok();
        }
        Ok(())
    }
}

#[derive(Args, Debug)]
pub struct Count;

impl Count {
    pub async fn run(self, client: &Client) -> Result<()> {
        let count = client.count().await?;
        println!("{count}");
        Ok(())
    }
}

#[derive(Args, Debug)]
pub struct Get {
    id: String,
}

impl Get {
    pub async fn run(self, client: &Client) -> Result<()> {
        let image = client.get(&self.id).await?;
        println!("{}", pretty(&image));
        Ok(())
    }
}

#[derive(Args, Debug)]
pub struct List;

impl List {
    pub async fn run(self, client: &Client) -> Result<()> {
        let mut images = pin!(client.list().await);
        while let Some(image) = images.next().await {
            println!("{}", pretty(&image));
        }
        Ok(())
    }
}

#[derive(Args, Debug)]
pub struct Download;

impl Download {
    pub async fn run(self) -> Result<()> {
        todo!();
    }
}

#[derive(Args, Debug)]
pub struct Upload {
    #[clap(flatten)]
    upload: UploadArgs,
    pub image: PathBuf,
}

impl Upload {
    pub async fn run(self, client: &Client) -> Result<()> {
        let upload = self.upload.update(crate::Upload::default());
        let (image, gyazo_id) = client.upload_image(&self.image, &upload).await?;
        println!("X-Gyazo-Id = {gyazo_id:?}");
        println!("{}", pretty(&image));
        println!("download: {}", image.download_url());
        Ok(())
    }
}
