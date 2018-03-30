extern crate threadpool;
extern crate reqwest;
extern crate select;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use std::collections::{HashMap, HashSet};
use std::hash;
use std::sync::{Arc, Mutex};

use select::document::Document;
use select::node::Node;
use select::predicate::{Attr, Class, Name, Predicate};

pub type ItemType = Arc<Movie>;
pub type ModelType = Arc<Mutex<HashMap<String, HashSet<ItemType>>>>;

#[derive(PartialEq, Eq, Serialize)]
pub struct Movie {
    pub name: String,
    pub actors: Vec<String>,
    pub year: Option<String>,
    pub director: Option<String>,
}

impl Movie {
    fn new(name: String) -> Movie {
        Movie {
            name,
            actors: vec![],
            year: None,
            director: None,
        }
    }

    fn index_terms(&self) -> Vec<String> {
        let mut terms = string_to_terms(self.name.as_str());
        for actor in self.actors.iter() {
            terms.extend_from_slice(string_to_terms(actor).as_slice());
        }
        for year in self.year.iter() {
            terms.extend_from_slice(string_to_terms(year).as_slice());
        }
        for director in self.director.iter() {
            terms.extend_from_slice(string_to_terms(director).as_slice())
        }
        terms
    }

    pub fn as_json_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

impl hash::Hash for Movie {
    fn hash<H>(&self, state: &mut H)
    where
        H: hash::Hasher,
    {
        self.name.hash(state);
    }
}


#[derive(Debug)]
pub enum CrawlerError {
    HttpError(reqwest::Error),
    FormatError(&'static str),
}

impl From<reqwest::Error> for CrawlerError {
    fn from(err: reqwest::Error) -> Self {
        CrawlerError::HttpError(err)
    }
}

pub struct Crawler {
    thread_pool: threadpool::ThreadPool,
}

impl Crawler {
    pub fn new(workers: usize) -> Crawler {
        let thread_pool = threadpool::ThreadPool::new(workers);
        Crawler { thread_pool }
    }

    pub fn crawl(&self) -> Result<ModelType, CrawlerError> {
        let hash_map = Arc::new(Mutex::new(HashMap::new()));
        let url_base = "http://www.imdb.com";
        let mut url = Some(format!(
            "{}/search/title{}",
            url_base,
            "?groups=top_1000&sort=user_rating&view=simple"
        ));
        // Walk through pages by visiting "next" and fetch the content of each
        // page in a seperate function executed by the thread pool.
        while let Some(page_url) = url.take() {
            let page_string = reqwest::get(page_url.as_str())?.text()?;
            let document = Document::from(page_string.as_str());
            for link_node in document.find(Class("lister-item-header").descendant(Name("a"))) {
                if let Some(link) = link_node.attr("href").map(&|s: &str| s.to_string()) {
                    let map = hash_map.clone();
                    let url = format!("{}{}", url_base, link);
                    self.thread_pool.execute(
                        move || match fetch_movie_detail(url, map) {
                            Ok(()) => (),
                            Err(e) => println!("Crawling error: {:?}", e),
                        },
                    );
                }
            }
            let mut next_page = document.find(Class("lister-page-next"));
            url = next_page
                .next()
                .map(&|node: Node| {
                    node.attr("href").map(&|s: &str| {
                        format!("{}/search/title{}", url_base, s)
                    })
                })
                .unwrap_or(None);

        }
        self.thread_pool.join();
        Ok(hash_map)
    }
}

fn fetch_movie_detail(page_url: String, map: ModelType) -> Result<(), CrawlerError> {
    println!("Fetching: {}", page_url);
    let page_string = reqwest::get(page_url.as_str())?.text()?;
    let document = Document::from(page_string.as_str());
    let title: String;
    if let Some(title_node) = document
        .find(Class("title_wrapper").descendant(Name("h1")))
        .next()
    {
        title = title_node.text().trim().to_string();
    } else {
        return Err(CrawlerError::FormatError("No title found"));
    }

    let mut movie = Movie::new(title);
    movie.year = document
        .find(Attr("id", "titleYear").descendant(Name("a")))
        .next()
        .map(&|n: Node| n.text());
    movie.director = document
        .find(Attr("itemprop", "creator").descendant(
            Attr("itemprop", "name"),
        ))
        .next()
        .map(&|n: Node| n.text().trim().to_string());
    for actor_node in document.find(Attr("itemprop", "actors").descendant(
        Attr("itemprop", "name"),
    ))
    {
        movie.actors.push(actor_node.text().trim().to_string());
    }

    let terms = movie.index_terms();
    let arc = Arc::new(movie);
    for k in terms {
        map.add_key_value(k.as_str(), &arc);
    }
    Ok(())
}

pub trait Storage<T: Clone> {
    fn add_key_value(&self, key: &str, value: &T);
    fn query(&self, key: &str) -> Vec<T>;
}

impl Storage<ItemType> for ModelType {
    fn add_key_value(&self, key: &str, value: &ItemType) {
        let mut map = self.lock().unwrap();
        map.entry(key.to_lowercase())
            .and_modify(|e| { e.insert(value.clone()); })
            .or_insert({
                let mut set = HashSet::new();
                set.insert(value.clone());
                set
            });
    }

    fn query(&self, key: &str) -> Vec<ItemType> {
        let map = self.lock().unwrap();
        match map.get(key) {
            Some(set) => set.iter().map(&|m: &ItemType| m.clone()).collect(),
            None => vec![],
        }
    }
}

fn string_to_terms(s: &str) -> Vec<String> {
    s.to_owned()
        .split_whitespace()
        .map(&|s: &str| s.to_owned())
        .collect()
}
