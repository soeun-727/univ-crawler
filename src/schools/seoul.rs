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

    // SWU 학사공지: iframe이 로드하는 실제 목록 엔드포인트
    let list_base = "https://www.swu.ac.kr/front/boardlist.do";
    let current_page = 1u32; // 필요 시 2,3페이지로 교체
    // bbsConfigFK=4 가 학사 게시판 (네가 캡처한 URL 기준)
    let url = format!(
        "{}?currentPage={}&menuGubun=1&siteGubun=1&bbsConfigFK=4&searchField=ALL&searchValue=&searchLowItem=ALL",
        list_base, current_page
    );

    let res = client.get(&url).header(REFERER, list_base).send()?;
    if !res.status().is_success() {
        eprintln!("HTTP {}", res.status());
        return Ok(Vec::new());
    }
    let body = res.text()?;
    let document = Document::from(body.as_str());

    let mut notices = Vec::new();

    // 행: table > tbody > tr (상단 고정 공지는 tr.notice)
    for tr in document.find(Name("table").descendant(Name("tbody")).descendant(Name("tr"))) {
        // 제목 a: td.title > div > a
        let a = tr
            .find(
                Name("td")
                    .and(Class("title"))
                    .descendant(Name("div"))
                    .descendant(Name("a")),
            )
            .next();

        // 제목: strong 우선, 없으면 a 전체 텍스트
        let title = a
            .as_ref()
            .and_then(|n| n.find(Name("strong")).next().or_else(|| n.find(Name("span")).next()))
            .map(|n| n.text().trim().to_string())
            .or_else(|| a.as_ref().map(|n| n.text().trim().to_string()))
            .unwrap_or_else(|| "(제목 없음)".to_string());

        // 날짜: tr 내 div.ls0 중 "게시일" 칸(일반적으로 td의 div.ls0)
        // 페이지 구조상 여러 ls0가 있을 수 있어 마지막/적절한 것을 선택
        let ls: Vec<_> = tr.find(Class("ls0")).collect();
        let date = ls
            .last()
            .map(|n| n.text().trim().to_string())
            .unwrap_or_else(|| "N/A".to_string());

        // 상세 URL:
        // a.href는 javascript:void(0); onclick이 핵심
        // onclick="boardMove('/front/boardview.do','<pkid>')"
        let detail_url = if let Some(onclick) = a.as_ref().and_then(|n| n.attr("onclick")) {
            if let Some(pkid) = parse_board_move_pkid(onclick) {
                format!("https://www.swu.ac.kr/front/boardview.do?pkid={}", pkid)
            } else {
                // 폴백: 디버깅용
                format!("javascript:{}", onclick)
            }
        } else if let Some(href) = a.as_ref().and_then(|n| n.attr("href")) {
            // 혹시 사이트가 href를 직접 제공할 때를 대비
            if href.starts_with("http") {
                href.to_string()
            } else {
                format!("https://www.swu.ac.kr{}", href)
            }
        } else {
            // 행 자체가 제목이 없거나 헤더일 수 있음 → 스킵
            continue;
        };

        // 빈 행/헤더 제외
        if !title.is_empty() {
            notices.push(Notice { title, date, url: detail_url });
        }
    }

    if notices.is_empty() {
        eprintln!("[SWU] parsed 0 notices — 선택자 또는 페이지 구조를 다시 확인하세요.");
    }

    Ok(notices)
}

/// onclick="boardMove('/front/boardview.do','506895')" 에서 pkid 추출
fn parse_board_move_pkid(onclick: &str) -> Option<String> {
    // 괄호 안 인자 추출
    let lp = onclick.find('(')? + 1;
    let rp = onclick[lp..].find(')')? + lp;
    let args = &onclick[lp..rp]; // 예: "'/front/boardview.do','506895'"

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