mod constants;
mod crawler;
mod utils;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    crawler::crawl_and_save("output.json").await
}
