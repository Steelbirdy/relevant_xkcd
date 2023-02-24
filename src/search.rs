use crate::crawler::ComicInfo;
use aho_corasick::AhoCorasick;
use rayon::prelude::*;

pub fn search<'a>(search: &Search, comics: &'a [ComicInfo<'a>]) -> Vec<SearchResult<'a>> {
    let results = std::sync::Mutex::new(Vec::new());
    comics.par_iter().for_each(|comic| {
        for &location in Location::ORDER {
            let loc = LocationFlags::from(location);
            if search.location.contains(loc)
                && search_in_location(comic, location, &search.aho_corasick)
            {
                {
                    let mut res = results.lock().unwrap();
                    res.push(SearchResult { comic, location });
                }
                return;
            }
        }
    });
    results.into_inner().unwrap()
}

fn search_in_location(comic: &ComicInfo, location: Location, aho: &AhoCorasick) -> bool {
    let Some(field) = location.get(comic)
    else { return false };
    aho.is_match(field)
}

bitflags::bitflags! {
    pub struct LocationFlags: u16 {
        const TITLE = 0b001;
        const TRANSCRIPT = 0b010;
        const ALT_TEXT = 0b100;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Location {
    Title,
    Transcript,
    AltText,
}

impl Location {
    const ORDER: &'static [Self] = &[Self::Title, Self::Transcript, Self::AltText];

    fn get<'a>(self, comic: &'a ComicInfo<'a>) -> Option<&'a str> {
        match self {
            Self::Title => Some(comic.title.as_ref()),
            Self::Transcript => comic.transcript.as_deref(),
            Self::AltText => comic.alt_text.as_deref(),
        }
    }
}

impl From<Location> for LocationFlags {
    fn from(loc: Location) -> Self {
        match loc {
            Location::Title => LocationFlags::TITLE,
            Location::Transcript => LocationFlags::TRANSCRIPT,
            Location::AltText => LocationFlags::ALT_TEXT,
        }
    }
}

pub struct SearchResult<'a> {
    pub comic: &'a ComicInfo<'a>,
    pub location: Location,
}

pub struct Search {
    aho_corasick: AhoCorasick,
    location: LocationFlags,
}

impl Search {
    pub fn new<I>(queries: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<[u8]>,
    {
        let aho = AhoCorasick::new(queries);
        Self {
            aho_corasick: aho,
            location: LocationFlags::all(),
        }
    }

    pub fn builder<T>() -> SearchBuilder<T> {
        SearchBuilder {
            queries: Vec::new(),
            location: LocationFlags::empty(),
        }
    }
}

pub struct SearchBuilder<T> {
    queries: Vec<T>,
    location: LocationFlags,
}

impl<T> SearchBuilder<T> {
    pub fn add_queries<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = T>,
    {
        self.queries.extend(iter);
        self
    }

    pub fn add_query(&mut self, q: T) -> &mut Self {
        self.queries.push(q);
        self
    }

    pub fn add_location(&mut self, loc: Location) -> &mut Self {
        self.location |= LocationFlags::from(loc);
        self
    }

    pub fn build(self) -> Search
    where
        T: AsRef<[u8]>,
    {
        let aho_corasick = aho_corasick::AhoCorasickBuilder::new()
            .ascii_case_insensitive(true)
            .build(self.queries);
        Search {
            aho_corasick,
            location: self.location,
        }
    }
}
