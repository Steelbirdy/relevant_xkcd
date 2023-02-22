mod constants;
mod crawler;



type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    let urls = get_comic_table_urls().await?;
    let mut all_comics = Vec::with_capacity(3000);
    for url in urls {
        let info = get_comic_info(&url).await?;
        all_comics.extend(info);
    }

    Ok(())
}

