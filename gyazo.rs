use gyazo::cli::Gyazo;

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    Gyazo::new().run().await?;
    Ok(())
}
