mod constants;
mod crawler;
mod utils;
mod search;

use crawler::ComicInfo;

use std::io::Write;
use rayon::prelude::*;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const FIELDS: &[&[ComicField]] = &[&[ComicField::Title], &[ComicField::Transcript, ComicField::AltText]];

#[tokio::main]
async fn main() -> Result<()> {
    // crawler::crawl_and_save("output.json").await

    let contents = std::fs::read_to_string("output.json")?;
    let info: Vec<ComicInfo> = serde_json::from_str(&contents)?;

    print!("Search for... ");
    std::io::stdout().flush()?;
    let line = std::io::stdin().lines().next().unwrap()?;
    let searcher = aho_corasick::AhoCorasickBuilder::new()
        .ascii_case_insensitive(true)
        .build([line]);

    let results = std::sync::RwLock::new(std::collections::HashSet::new());
    for &fields in FIELDS {
        info.par_iter()
            .for_each(|comic| {
                {
                    let res = results.read().unwrap();
                    if res.contains(&comic.index) {
                        return;
                    }
                }
                for field in fields {
                    if let Some(field) = field.get(comic) {
                        if searcher.is_match(field) {
                            eprintln!("Found match: #{} - {} @ {}", comic.index, &*comic.title, &*comic.xkcd_url);
                            {
                                let mut res = results.write().unwrap();
                                res.insert(comic.index);
                            }
                            return;
                        }
                    }
                }
            })
    }

    Ok(())
}

#[derive(Debug, Copy, Clone)]
enum ComicField {
    Title,
    Transcript,
    AltText,
}

impl ComicField {
    fn get<'a>(self, comic: &'a ComicInfo<'a>) -> Option<&'a str> {
        match self {
            Self::Title => Some(&*comic.title),
            Self::Transcript => comic.transcript.as_deref(),
            Self::AltText => comic.alt_text.as_deref(),
        }
    }
}
