use crate::{constants, utils::CowStr, Result};
use reqwest::Client;
use select::{document::Document, predicate::*};
use serde::{Deserialize, Serialize};
use std::{fs::File, path::Path};

const CHUNK_SIZE: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComicInfo<'a> {
    pub index: u32,
    #[serde(borrow)]
    pub title: CowStr<'a>,
    pub transcript: Option<CowStr<'a>>,
    pub alt_text: Option<CowStr<'a>>,
    pub wiki_url: CowStr<'a>,
    pub xkcd_url: CowStr<'a>,
    pub image_url: CowStr<'a>,
}

pub async fn crawl_and_save<P: AsRef<Path>>(path: P) -> Result<()> {
    let file = File::create(path)?;
    crawl_and_save_inner(file).await
}

async fn crawl_and_save_inner(file: File) -> Result<()> {
    let client = Client::new();
    let urls = get_comic_table_urls(&client).await?;

    let mut comic_data = Vec::with_capacity(3000);
    for url in urls {
        let data = get_comic_table_data(&client, &url).await?;
        comic_data.extend(data);
    }

    let num_comics = comic_data.len();
    let progress = indicatif::ProgressBar::new(num_comics as _);

    let mut pool = tokio::task::JoinSet::new();

    while !comic_data.is_empty() {
        let start = comic_data.len() - comic_data.len().min(CHUNK_SIZE);
        let chunk = comic_data.split_off(start);

        let client = client.clone();
        let progress = progress.clone();
        pool.spawn(async move {
            let mut ret = Vec::with_capacity(chunk.len());
            for comic in chunk {
                let info = get_comic_info(&client, comic).await?;
                progress.inc(1);
                ret.push(info);
            }
            Result::Ok(ret)
        });
    }

    let mut comics = Vec::with_capacity(num_comics);
    while let Some(c) = pool.join_next().await {
        let c = c.unwrap()?;
        comics.extend(c);
    }
    progress.finish();

    comics.sort_by_key(|comic| comic.index);
    serde_json::to_writer(file, &comics)?;
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

async fn get_comic_table_urls(client: &Client) -> Result<Vec<String>> {
    let res = client
        .get(constants::FULL_COMICS_LIST_URL)
        .send()
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

    Ok(nodes
        .map(move |node| {
            let link = extract_link(&node);
            format!("{}{link}", constants::URL_BASE)
        })
        .collect())
}

async fn get_comic_table_data(client: &Client, table_url: &str) -> Result<Vec<ComicData>> {
    let res = client.get(table_url).send().await?.text().await?;

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
        let (c1, c2, _, c4) = (
            cells.next().unwrap(),
            cells.next().unwrap(),
            cells.next(),
            cells.next().unwrap(),
        );

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

async fn get_comic_info(client: &Client, comic: ComicData) -> Result<ComicInfo<'static>> {
    let res = client
        .get(format!("{}{}", constants::URL_BASE, &comic.wiki_url))
        .send()
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
        .take_while(|text| {
            !text.is_empty()
                && !text.ends_with("[edit]")
                && !text.chars().all(|c| c.is_whitespace())
        })
        .reduce(|a, b| format!("{a}\n{b}"));

    let ComicData {
        index,
        title,
        wiki_url,
        xkcd_url,
        image_url,
    } = comic;
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
