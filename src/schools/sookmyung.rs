// src/schools/sookmyung.rs
use reqwest::blocking::Client;
use reqwest::header::REFERER;
use select::document::Document;
use select::predicate::{Name, Class, Predicate};
use std::error::Error;
use std::time::Duration;

pub struct Notice {
    pub title: String,
    pub date: String,
    pub url: String,
}

pub fn fetch_notices() -> Result<Vec<Notice>, Box<dyn Error>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(10))
        .build()?;

    let base_url = "https://www.sookmyung.ac.kr/kr/news/important-notice.do";
    let article_limit = 10;
    let offset = 0;

    let url = format!(
        "{}?mode=list&articleLimit={}&article.offset={}",
        base_url, article_limit, offset
    );

    let res = match client.get(&url).header(REFERER, base_url).send() {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            eprintln!("Failed to fetch page: HTTP {}", r.status());
            return Ok(Vec::new());
        }
        Err(e) => {
            eprintln!("Network request failed: {}", e);
            return Ok(Vec::new());
        }
    };

    let body = match res.text() {
        Ok(text) => text,
        Err(e) => {
            eprintln!("Failed to read response text: {}", e);
            return Ok(Vec::new());
        }
    };

    let document = Document::from(body.as_str());
    let mut notices = Vec::new();

    for tr in document.find(Name("table").descendant(Name("tbody")).descendant(Name("tr"))) {
        let title = tr.find(Class("b-td-title"))
            .next()
            .and_then(|td| td.find(Name("a")).next())
            .map(|a| a.text().trim().to_string())
            .unwrap_or_else(|| {
                eprintln!("Warning: Failed to parse title");
                "(제목 없음)".to_string()
            });

        let date = tr.find(Class("b-date-box").or(Class("b-date")))
            .next()
            .map(|d| d.text().trim().to_string())
            .unwrap_or_else(|| {
                eprintln!("Warning: Failed to parse date");
                "N/A".to_string()
            });

        let url = tr.find(Class("b-td-title"))
            .next()
            .and_then(|td| td.find(Name("a")).next())
            .and_then(|a| a.attr("href"))
            .map(|s| format!("https://www.sookmyung.ac.kr/kr/news/important-notice.do{}", s))
            .unwrap_or_default();

        notices.push(Notice { title, date, url });
    }

    Ok(notices)
}

// RSS 생성 함수
use rss::{ChannelBuilder, ItemBuilder};

pub fn create_rss(notices: &[Notice]) -> rss::Channel {
    let items = notices.iter().map(|notice| {
        ItemBuilder::default()
            .title(notice.title.clone())
            .link(notice.url.clone())
            .pub_date(notice.date.clone())
            .build()  
    }).collect::<Vec<_>>();

    let channel = ChannelBuilder::default()
        .title("숙명여자대학교 공지 RSS")
        .link("https://www.sookmyung.ac.kr/kr/news/important-notice.do")
        .description("숙명여대 주요 공지 RSS 피드")
        .items(items)
        .build(); 

    channel
}
