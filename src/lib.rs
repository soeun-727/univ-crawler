//src/lib.rs
use reqwest::blocking::Client;
use reqwest::header::REFERER;
use select::document::Document;
use select::predicate::{Name, Class, Predicate};
use std::error::Error;

//as a function
pub fn fetch_notices() -> Result<(), Box<dyn Error>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36")
        .build()?;

    let base_url = "https://www.sookmyung.ac.kr/kr/news/important-notice.do";
    let article_limit = 10;
    let offset = 0; //just the first page

    let url = format!(
        "{}?mode=list&articleLimit={}&article.offset={}",
        base_url, article_limit, offset
    );
    println!("Fetching first page with offset = {}", offset);

    let res = client
        .get(&url)
        .header(REFERER, base_url)
        .send()?;

    if !res.status().is_success() {
        eprintln!("Failed to fetch page: HTTP {}", res.status());
        return Ok(());
    }

    let body = res.text()?;
    let document = Document::from(body.as_str());

    for tr in document.find(Name("table").descendant(Name("tbody")).descendant(Name("tr"))) {
        let _number = tr
            .find(Class("b-num-box"))
            .next()
            .map(|n| n.text().trim().to_string())
            .unwrap_or_default();

        let (title, _article_no) = if let Some(a) = tr.find(Class("b-td-title")).next()
            .and_then(|td| td.find(Name("a")).next())
        {
            let title = a.text().trim().to_string();
            let article_no = a.attr("data-article-no").unwrap_or("").to_string();
            (title, article_no)
        } else {
            ("".to_string(), "".to_string())
        };

        let date = tr
            .find(Class("b-date-box"))
            .next()
            .map(|d| d.text().trim().to_string())
            .unwrap_or_default();

        println!("제목: {}, [작성일: {}]", title, date);
    }

    Ok(())
}
