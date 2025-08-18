use reqwest::blocking::Client;
use reqwest::header::REFERER;
use select::document::Document;
use select::predicate::{Class, Name, Predicate};
use std::error::Error;
use std::time::Duration;

// 공통 Notice 타입 재사용
use crate::schools::sookmyung;
pub type Notice = sookmyung::Notice;

/// "YYYY.MM.DD / YYYY-MM-DD / YYYY/MM/DD" 형태 감지
fn is_date_like(s: &str) -> bool {
    let t = s.trim().trim_end_matches('.');
    let sep = if t.contains('.') { '.' } else if t.contains('-') { '-' } else if t.contains('/') { '/' } else { return false };
    let parts: Vec<_> = t.split(sep).map(|x| x.trim()).collect();
    if parts.len() != 3 { return false; }
    let (y, m, d) = (parts[0], parts[1], parts[2]);
    y.len() == 4 && y.chars().all(|c| c.is_ascii_digit())
        && m.chars().all(|c| c.is_ascii_digit())
        && d.chars().all(|c| c.is_ascii_digit())
}

/// 동덕여대 학사 공지 1페이지 수집
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
        // 제목: dt > a.subTit
        let a = li
            .find(Name("dt").descendant(Name("a").and(Class("subTit"))))
            .next();

        let title = a
            .as_ref()
            .map(|n| n.text().trim().to_string())
            .unwrap_or_else(|| "(제목 없음)".to_string());

        // 날짜: li 내부 텍스트 중 "날짜처럼 보이는" 것만 추출 (조회수/댓글수 등 제외)
        let mut date = String::new();

        // 우선 dd > .p_hide 스캔
        for node in li.find(Name("dd").descendant(Class("p_hide"))) {
            let t = node.text();
            if is_date_like(&t) { date = t.trim().to_string(); break; }
        }
        // 여전히 비었으면 다른 셀들에서도 탐색
        if date.is_empty() {
            for node in li.find(Name("span").or(Name("dd")).or(Name("div"))) {
                let t = node.text();
                if is_date_like(&t) { date = t.trim().to_string(); break; }
            }
        }
        if date.is_empty() { date = "N/A".to_string(); }

        // 상세 URL: href의 schM=view 우선, 없으면 onclick="fn_goView('id', false, 'no', '')" 파싱
        let detail_url = if let Some(href) = find_view_href_in_li(&li) {
            href
        } else {
            let onclick = a.as_ref().and_then(|n| n.attr("onclick")).unwrap_or("");
            if let Some((id, no)) = parse_fn_go_view(onclick) {
                format!("{base_url}?schM=view&id={id}&etc1={no}")
            } else {
                // 폴백(디버깅용)
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
                .pub_date(n.date.clone()) // RFC 변환은 main에서 normalize
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

/* ─── 유틸 ─── */

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
    // 예: fn_goView('90378', false, '8901', '')
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
