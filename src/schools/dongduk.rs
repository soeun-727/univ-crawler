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

    // 동덕여대 목록: ?schM=list&page=&viewCount=
    let base_url = "https://www.dongduk.ac.kr/www/contents/kor-noti.do";
    let page = 1u32;
    let view_count = 10u32;

    let url = format!("{}?schM=list&page={}&viewCount={}", base_url, page, view_count);

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

    // 공지 1건 = ul.board-basic > li
    for li in document.find(Class("board-basic").descendant(Name("li"))) {
        // 제목/onclick 보유한 a.subTit
        let a_subtit = li
            .find(Name("dt").descendant(Name("a").and(Class("subTit"))))
            .next();

        // 제목: a.subTit 텍스트에서 선두 [카테고리] 제거 (문자열 조작)
        let title = a_subtit
            .as_ref()
            .map(|node| node.text())
            .map(|t| strip_category_prefix(&t))
            .unwrap_or_else(|| {
                eprintln!("Warning: Failed to parse title");
                "(제목 없음)".to_string()
            });

        // 날짜: dd > span.p_hide (마지막)
        let date = li
            .find(Name("dd").descendant(Class("p_hide")))
            .collect::<Vec<_>>()
            .last()
            .map(|d| d.text().trim().to_string())
            .unwrap_or_else(|| {
                eprintln!("Warning: Failed to parse date");
                "N/A".to_string()
            });

        // 상세 URL 우선순위:
        // 1) li 내부의 어떤 <a>든 href에 schM=view가 있으면 그걸 사용(상대경로면 base 붙이기)
        // 2) 없으면 a.subTit의 onclick에서 id/no 추출 후 ?schM=view&id=..&etc1=.. 조합
        let detail_url = find_view_href_in_li(&li)
            .or_else(|| {
                let onclick = a_subtit
                    .as_ref()
                    .and_then(|node| node.attr("onclick"))
                    .unwrap_or("");
                let (id, no) = parse_fn_go_view(onclick);
                if !id.is_empty() && !no.is_empty() {
                    Some(format!("{}?schM=view&id={}&etc1={}", base_url, id, no))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                // 마지막 폴백: onclick 원문을 남겨 디버깅
                let onclick = a_subtit
                    .as_ref()
                    .and_then(|node| node.attr("onclick"))
                    .unwrap_or("");
                format!("javascript:{}", onclick)
            });

        notices.push(Notice { title, date, url: detail_url });
    }

    Ok(notices)
}

/// 문자열 선두의 "[...]" 블럭과 그 뒤 공백 제거 (정규식 없이)
fn strip_category_prefix(s: &str) -> String {
    let s = s.trim();
    if !s.starts_with('[') {
        return s.to_string();
    }
    if let Some(end) = s.find(']') {
        let after = &s[(end + 1)..];
        return after.trim_start().to_string();
    }
    s.to_string()
}

/// li 내부에서 href에 "schM=view"가 들어간 a 태그를 찾아 절대 URL로 반환
fn find_view_href_in_li(li: &select::node::Node) -> Option<String> {
    for a in li.find(Name("a")) {
        if let Some(href) = a.attr("href") {
            if href.contains("schM=view") {
                if href.starts_with("http") {
                    return Some(href.to_string());
                } else {
                    // 상대경로 처리
                    return Some(format!("https://www.dongduk.ac.kr{}", href));
                }
            }
        }
    }
    None
}

/// onclick="fn_goView('id', something, 'no', ...)" 에서 id/no 추출 (정규식 없이)
fn parse_fn_go_view(onclick: &str) -> (String, String) {
    let marker = "fn_goView(";
    let start = match onclick.find(marker) {
        Some(i) => i + marker.len(),
        None => return (String::new(), String::new()),
    };
    let end = match onclick[start..].find(')') {
        Some(i) => start + i,
        None => return (String::new(), String::new()),
    };
    let args_str = &onclick[start..end]; // 예: '90378', false, '8901', ''

    let mut parts: Vec<String> = Vec::new();
    for raw in args_str.split(',') {
        let t = raw.trim();
        // 양쪽 홑따옴표만 제거
        let cleaned = t.trim_matches('\'').to_string();
        parts.push(cleaned);
    }

    // 기대: [ id, notice, no, secret ]
    if parts.len() >= 3 {
        let id = parts[0].clone();
        let no = parts[2].clone();
        (id, no)
    } else {
        (String::new(), String::new())
    }
}

// RSS 생성
use rss::{ChannelBuilder, ItemBuilder};

pub fn create_rss(notices: &[Notice]) -> rss::Channel {
    let items = notices
        .iter()
        .map(|notice| {
            ItemBuilder::default()
                .title(notice.title.clone())
                .link(notice.url.clone())
                .pub_date(notice.date.clone())
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

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn print_first_page() {
//         let notices = fetch_notices().expect("fetch_notices failed");
//         assert!(notices.len() > 0, "no notices parsed");
//         for n in notices.iter().take(5) {
//             println!("{} | {} | {}", n.date, n.title, n.url);
//         }
//     }
// }