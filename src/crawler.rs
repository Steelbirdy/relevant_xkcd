use select::{document::Document, predicate::*};
use crate::{constants, Result, utils::CowStr};
use std::{path::Path, fs::File};
use std::sync::Arc;
use indicatif::MultiProgress;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComicInfo<'a> {
    index: u32,
    #[serde(borrow)]
    title: CowStr<'a>,
    transcript: Option<CowStr<'a>>,
    alt_text: Option<CowStr<'a>>,
    wiki_url: CowStr<'a>,
    xkcd_url: CowStr<'a>,
    image_url: CowStr<'a>,
}

pub async fn crawl_and_save<P: AsRef<Path>>(path: P) -> Result<()> {
    async fn task(url: String, progress: Arc<MultiProgress>) -> Result<Vec<ComicInfo<'static>>> {
        let comics = get_comic_table_data(&url).await?;
        let bar = indicatif::ProgressBar::new(comics.len() as _);
        let bar = progress.add(bar);
        let mut ret = Vec::with_capacity(comics.len());
        for comic in comics {
            let info = get_comic_info(comic).await?;
            ret.push(info);
            bar.inc(1);
        }
        bar.finish();
        Ok(ret)
    }

    let file = File::create(path)?;

    let urls = get_comic_table_urls().await?;
    let mut threads = tokio::task::JoinSet::new();
    let progress = Arc::new(MultiProgress::new());
    for url in urls {
        threads.spawn(task(url, progress.clone()));
    }

    let mut all_comics = Vec::with_capacity(3000);
    while let Some(res) = threads.join_next().await {
        let res = res.unwrap()?;
        all_comics.extend(res);
    }

    all_comics.sort_by_key(|comic| comic.index);
    serde_json::to_writer(file, &all_comics)?;
    Ok(())
}

#[derive(Debug)]
struct ComicData {
    index: u32,
    title: String,
    wiki_url: String,
    xkcd_url: String,
    image_url: String,
}

async fn get_comic_table_urls() -> Result<Vec<String>> {
    let res = reqwest::get(constants::FULL_COMICS_LIST_URL)
        .await?
        .text()
        .await?;

    let predicate = Name("body")
        .descendant(Attr("id", "mw-content-text"))
        .child(Class("mw-parser-output"))
        .child(Name("dl"))
        .child(Name("dd"));
    let doc = Document::from(res.as_str());
    let nodes = doc.find(predicate);

    Ok(nodes.map(move |node| {
        let link = extract_link(&node);
        format!("{}{link}", constants::URL_BASE)
    }).collect())
}

async fn get_comic_table_data(table_url: &str) -> Result<Vec<ComicData>> {
    let res = reqwest::get(table_url)
        .await?
        .text()
        .await?;

    let predicate = Name("body")
        .descendant(Attr("id", "mw-content-text"))
        .child(Class("mw-parser-output"))
        .child(Name("table"))
        .descendant(Name("tr"));
    let doc = Document::from(res.as_str());
    let rows = doc.find(predicate).skip(1);

    let mut ret = Vec::new();
    for row in rows {
        let mut cells = row.find(Name("td"));
        let (c1, c2, _, c4) = (cells.next().unwrap(), cells.next().unwrap(), cells.next(), cells.next().unwrap());

        let (xkcd_url, index) = {
            let text = c1.text();
            let text = text.trim();
            let index = text.rsplit_once('/').unwrap().1.parse().unwrap();
            let url = format!("https://{text}");
            (url, index)
        };

        let c2 = c2.find(Name("a")).next().unwrap();
        let title = c2.text();

        let wiki_url = extract_link(&c2);

        let image_url = extract_link(&c4);

        ret.push(ComicData {
            index,
            title,
            wiki_url: wiki_url.to_string(),
            xkcd_url,
            image_url: image_url.to_string(),
        });
    }

    Ok(ret)
}

async fn get_comic_info(comic: ComicData) -> Result<ComicInfo<'static>> {
    let res = reqwest::get(format!("{}{}", constants::URL_BASE, &comic.wiki_url))
        .await?
        .text()
        .await?;

    let alt_text_predicate = Name("body")
        .descendant(Attr("id", "mw-content-text"))
        .child(Class("mw-parser-output"))
        .child(Name("table"))
        .descendant(Attr("href", comic.image_url.as_str()));
    let doc = Document::from(res.as_str());
    let alt_text = doc
        .find(alt_text_predicate)
        .next()
        .and_then(|node| node.attr("title"))
        .map(ToString::to_string);

    let transcript_predicate = Name("body")
        .descendant(Attr("id", "mw-content-text"))
        .child(Class("mw-parser-output"))
        .child(Or(Or(Name("h2"), Name("dl")), Name("p")));
    let transcript = doc
        .find(transcript_predicate)
        .map(|dl| dl.text())
        .skip_while(|text| text != "Transcript[edit]")
        .skip(1)
        .take_while(|text| !text.is_empty() && !text.ends_with("[edit]") && !text.chars().all(|c| c.is_whitespace()))
        .reduce(|a, b| format!("{a}\n{b}"));

    let ComicData { index, title, wiki_url, xkcd_url, image_url } = comic;
    Ok(ComicInfo {
        index,
        title: title.into(),
        transcript: transcript.map(Into::into),
        alt_text: alt_text.map(Into::into),
        wiki_url: wiki_url.into(),
        xkcd_url: xkcd_url.into(),
        image_url: image_url.into(),
    })
}

#[track_caller]
fn extract_link<'a>(node: &'a select::node::Node) -> &'a str {
    if let Some(ret) = node.attr("href") {
        return ret;
    }

    node.find(Attr("href", ()))
        .next()
        .and_then(|node| node.attr("href"))
        .unwrap_or_else(|| panic!("attempted to extract link from node: `{}`", node.html()))
}