mod constants;
pub mod crawler;
pub mod search;
mod utils;

pub use crawler::ComicInfo;

use std::io::Write;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    // crawler::crawl_and_save("output.json").await?;

    let contents = std::fs::read_to_string("output.json")?;
    let comics: Vec<ComicInfo> = serde_json::from_str(&contents)?;

    print!("Search for... ");
    std::io::stdout().flush()?;
    let line = std::io::stdin().lines().next().unwrap()?;

    let search = search::Search::new([line]);
    for result in search::search(&search, &comics) {
        let comic = result.comic;
        eprintln!(
            r#"#{}: "{}" @ {}"#,
            comic.index,
            comic.title.as_ref(),
            comic.xkcd_url.as_ref()
        );
    }

    Ok(())
}
