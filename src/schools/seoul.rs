use reqwest::blocking::Client;
use reqwest::header::REFERER;
use select::document::Document;
use select::predicate::{Class, Name, Predicate};
use std::error::Error;
use std::time::Duration;

// 공통 Notice 타입 재사용
use crate::schools::sookmyung;
pub type Notice = sookmyung::Notice;

/// 서울여대 학사 공지 1페이지 수집 (제목 가공 없음)
pub fn fetch_notices() -> Result<Vec<Notice>, Box<dyn Error>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(10))
        .build()?;

    // iframe이 로드하는 실제 목록
    let list_base = "https://www.swu.ac.kr/front/boardlist.do";
    let url = format!("{list_base}?currentPage=1&menuGubun=1&siteGubun=1&bbsConfigFK=4&searchField=ALL&searchValue=&searchLowItem=ALL");

    let res = client.get(&url).header(REFERER, list_base).send()?;
    if !res.status().is_success() {
        eprintln!("[SWU] HTTP {}", res.status());
        return Ok(Vec::new());
    }
    let body = res.text()?;
    let document = Document::from(body.as_str());

    let mut notices = Vec::new();

    // 행: table > tbody > tr (상단 고정 공지는 tr.notice)
    for tr in document.find(Name("table").descendant(Name("tbody")).descendant(Name("tr"))) {
        // 제목 a: td.title > div > a (가공 없이 전체 텍스트)
        let a = tr
            .find(
                Name("td")
                    .and(Class("title"))
                    .descendant(Name("div"))
                    .descendant(Name("a")),
            )
            .next();

        let title = a
            .as_ref()
            .map(|n| n.text().trim().to_string())
            .unwrap_or_else(|| "(제목 없음)".to_string());

        // 날짜: tr 내 div.ls0 중 마지막(일반적으로 게시일)
        let ls: Vec<_> = tr.find(Class("ls0")).collect();
        let date = ls
            .last()
            .map(|n| n.text().trim().to_string())
            .unwrap_or_else(|| "N/A".to_string());

        // 상세 URL: onclick="boardMove('/front/boardview.do','<pkid>')"
        let detail_url = if let Some(onclick) = a.as_ref().and_then(|n| n.attr("onclick")) {
            if let Some(pkid) = parse_board_move_pkid(onclick) {
                format!("https://www.swu.ac.kr/front/boardview.do?pkid={pkid}")
            } else {
                format!("javascript:{onclick}") // 폴백
            }
        } else if let Some(href) = a.as_ref().and_then(|n| n.attr("href")) {
            if href.starts_with("http") {
                href.to_string()
            } else {
                format!("https://www.swu.ac.kr{href}")
            }
        } else {
            continue;
        };

        if !title.is_empty() {
            notices.push(Notice { title, date, url: detail_url });
        }
    }

    if notices.is_empty() {
        eprintln!("[SWU] parsed 0 notices — 선택자 또는 페이지 구조를 다시 확인하세요.");
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
        .title("서울여자대학교 학사 공지 RSS")
        .link("https://www.swu.ac.kr/www/noticea.html")
        .description("서울여대 학사 공지 RSS 피드")
        .items(items)
        .build()
}

/* ─── 아래는 비공개 유틸 ─── */

fn parse_board_move_pkid(onclick: &str) -> Option<String> {
    // 예: "boardMove('/front/boardview.do','506895')"
    let lp = onclick.find('(')? + 1;
    let rp = onclick[lp..].find(')')? + lp;
    let args = &onclick[lp..rp];

    let parts: Vec<String> = args
        .split(',')
        .map(|s| s.trim().trim_matches('\'').to_string())
        .collect();

    if parts.len() >= 2 {
        Some(parts[1].clone()) // pkid
    } else {
        None
    }
}


// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_fetch_notices() {
//         match fetch_notices() {
//             Ok(notices) => {
//                 assert!(
//                     !notices.is_empty(),
//                     "[SWU] 크롤링 결과가 비어있습니다."
//                 );
//                 for notice in notices.iter().take(5) {
//                     println!("제목: {}", notice.title);
//                     println!("날짜: {}", notice.date);
//                     println!("URL: {}\n", notice.url);
//                 }
//             }
//             Err(e) => panic!("[SWU] 크롤링 중 오류 발생: {}", e),
//         }
//     }
// }