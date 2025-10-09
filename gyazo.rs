use gyazo::{Result, cli::Gyazo};

#[tokio::main]
async fn main() -> Result<()> {
    Gyazo::new().run().await
}
