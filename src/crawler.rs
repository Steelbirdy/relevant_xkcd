use select::{document::Document, predicate::*};
use crate::{constants, Result};
use std::{path::Path, io::BufWriter, fs::File};
use serde::{Serialize, Deserialize};

pub async fn crawl_and_save<P: AsRef<Path>>(path: P) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    let urls = get_comic_table_urls().await?;
    let mut all_comics = Vec::with_capacity(3000);
    // for url in urls {
    //     let comics = get_comic_info(&url).await?;
    //     for comic in comics {
    //
    //     }
    // }

    todo!()
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ComicInfoRef<'a> {
    index: u32,
    title: &'a str,
    wiki_url: &'a str,
    xkcd_url: &'a str,
    image_url: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ComicInfo {
    index: u32,
    title: String,
    wiki_url: String,
    xkcd_url: String,
    image_url: String,
}

async fn get_comic_info(table_url: &str) -> Result<Vec<ComicInfo>> {
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
        let wiki_url = format!("{}{wiki_url}", constants::URL_BASE);

        let image_url = extract_link(&c4);
        let image_url = format!("{}{image_url}", constants::URL_BASE);

        ret.push(ComicInfo {
            index,
            title,
            wiki_url,
            xkcd_url,
            image_url,
        });
    }

    Ok(ret)
}

#[track_caller]
fn extract_link<'a>(node: &'a select::node::Node) -> &'a str {
    if let Some(ret) = node.attr("href") {
        return ret;
    }

    fn inner<'a>(node: &'a select::node::Node) -> Option<&'a str> {
        node.find(Attr("href", ()))
            .next()?
            .attr("href")
    }

    inner(node)
        .unwrap_or_else(|| panic!("attempted to extract link from node: `{}`", node.html()))
}