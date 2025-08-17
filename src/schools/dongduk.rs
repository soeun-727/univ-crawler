use reqwest::blocking::Client;
use reqwest::header::REFERER;
use select::document::Document;
use select::predicate::{Class, Name, Predicate};
use std::error::Error;
use std::time::Duration;

// 공통 Notice 타입 재사용
use crate::schools::sookmyung;
pub type Notice = sookmyung::Notice;

/// 동덕여대 학사 공지 1페이지 수집 (제목 가공 없음)
pub fn fetch_notices() -> Result<Vec<Notice>, Box<dyn Error>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(10))
        .build()?;

    let base_url = "https://www.dongduk.ac.kr/www/contents/kor-noti.do";
    let url = format!("{base_url}?schM=list&page=1&viewCount=10");

    let res = client.get(&url).header(REFERER, base_url).send()?;
    if !res.status().is_success() {
        eprintln!("[DONGDUK] HTTP {}", res.status());
        return Ok(Vec::new());
    }
    let body = res.text()?;
    let document = Document::from(body.as_str());

    let mut notices = Vec::new();

    // 공지 1건 = ul.board-basic > li
    for li in document.find(Class("board-basic").descendant(Name("li"))) {
        // 제목: a.subTit 전체 텍스트(가공 없음)
        let a = li
            .find(Name("dt").descendant(Name("a").and(Class("subTit"))))
            .next();

        let title = a
            .as_ref()
            .map(|n| n.text().trim().to_string())
            .unwrap_or_else(|| "(제목 없음)".to_string());

        // 날짜: dd > span.p_hide (마지막)
        let date = li
            .find(Name("dd").descendant(Class("p_hide")))
            .collect::<Vec<_>>()
            .last()
            .map(|d| d.text().trim().to_string())
            .unwrap_or_else(|| "N/A".to_string());

        // 상세 URL: href의 schM=view 우선, 없으면 onclick 파싱
        let detail_url = if let Some(href) = find_view_href_in_li(&li) {
            href
        } else {
            let onclick = a.as_ref().and_then(|n| n.attr("onclick")).unwrap_or("");
            if let Some((id, no)) = parse_fn_go_view(onclick) {
                format!("{base_url}?schM=view&id={id}&etc1={no}")
            } else {
                // 마지막 폴백(디버깅용)
                format!("javascript:{onclick}")
            }
        };

        if !title.is_empty() {
            notices.push(Notice { title, date, url: detail_url });
        }
    }

    Ok(notices)
}

// RSS 생성
use rss::{ChannelBuilder, ItemBuilder};

pub fn create_rss(notices: &[Notice]) -> rss::Channel {
    let items = notices
        .iter()
        .map(|n| {
            ItemBuilder::default()
                .title(n.title.clone())
                .link(n.url.clone())
                .pub_date(n.date.clone())
                .build()
        })
        .collect::<Vec<_>>();

    ChannelBuilder::default()
        .title("동덕여자대학교 학사 공지 RSS")
        .link("https://www.dongduk.ac.kr/www/contents/kor-noti.do?schM=list")
        .description("동덕여대 학사 공지 RSS 피드")
        .items(items)
        .build()
}

/* ─── 아래는 비공개 유틸 ─── */

fn find_view_href_in_li(li: &select::node::Node) -> Option<String> {
    for a in li.find(Name("a")) {
        if let Some(href) = a.attr("href") {
            if href.contains("schM=view") {
                return Some(if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("https://www.dongduk.ac.kr{href}")
                });
            }
        }
    }
    None
}

fn parse_fn_go_view(onclick: &str) -> Option<(String, String)> {
    // onclick="fn_goView('90378', false, '8901', '')"
    let marker = "fn_goView(";
    let start = onclick.find(marker)? + marker.len();
    let end = onclick[start..].find(')')? + start;
    let args = &onclick[start..end];

    let parts: Vec<String> = args
        .split(',')
        .map(|s| s.trim().trim_matches('\'').to_string())
        .collect();

    if parts.len() >= 3 {
        Some((parts[0].clone(), parts[2].clone())) // (id, no)
    } else {
        None
    }
}


// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn print_dong_page() {
//         let notices = fetch_notices().expect("fetch_notices failed");
//         assert!(notices.len() > 0, "no notices parsed");
//         for n in notices.iter().take(5) {
//             println!("{} | {} | {}", n.date, n.title, n.url);
//         }
//     }
// }