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
    let sep = if t.contains('.') {
        '.'
    } else if t.contains('-') {
        '-'
    } else if t.contains('/') {
        '/'
    } else {
        return false;
    };
    let parts: Vec<_> = t.split(sep).map(|x| x.trim()).collect();
    if parts.len() != 3 {
        return false;
    }
    let (y, m, d) = (parts[0], parts[1], parts[2]);
    y.len() == 4
        && y.chars().all(|c| c.is_ascii_digit())
        && m.chars().all(|c| c.is_ascii_digit())
        && d.chars().all(|c| c.is_ascii_digit())
}

/// 서울여대 학사 공지 1페이지 수집
pub fn fetch_notices() -> Result<Vec<Notice>, Box<dyn Error>> {
    let client = Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
             AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/114.0.0.0 Safari/537.36",
        )
        .timeout(Duration::from_secs(10))
        .build()?;

    // 리스트(iframe 실제 요청) – 학사공지: bbsConfigFK=4
    let list_base = "https://www.swu.ac.kr/front/boardlist.do";
    let site_gubun = 1;
    let menu_gubun = 1;
    let bbs_config_fk = 4;
    let url = format!(
        "{list_base}?currentPage=1&menuGubun={menu_gubun}&siteGubun={site_gubun}&bbsConfigFK={bbs_config_fk}&searchField=ALL&searchValue=&searchLowItem=ALL"
    );

    let res = client.get(&url).header(REFERER, list_base).send()?;
    if !res.status().is_success() {
        eprintln!("[SWU] HTTP {}", res.status());
        return Ok(Vec::new());
    }
    let body = res.text()?;
    let document = Document::from(body.as_str());

    let mut notices = Vec::new();

    // 행: table > tbody > tr
    for tr in document
        .find(Name("table").descendant(Name("tbody")).descendant(Name("tr")))
    {
        // 제목 a: td.title > div > a
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

        // 날짜: 행 내부 텍스트 중 "날짜처럼 보이는" 것만 추출 (조회수/댓글수 제외)
        let mut date = String::new();
        for cand in tr.find(Name("td").or(Name("div")).or(Name("span"))) {
            let t = cand.text();
            if is_date_like(&t) {
                date = t.trim().to_string();
                break;
            }
        }
        if date.is_empty() {
            date = "N/A".to_string();
        }

        // 상세 URL 생성:
        // onclick="boardMove('/front/boardview.do','<pkid>')" 에서 pkid 추출 후,
        // 컨텍스트 파라미터를 모두 포함한 GET 링크로 구성
        let detail_url = if let Some(onclick) = a.as_ref().and_then(|n| n.attr("onclick")) {
            if let Some(pkid) = parse_board_move_pkid(onclick) {
                // 필요한 컨텍스트를 전부 포함
                format!(
                    "https://www.swu.ac.kr/front/boardview.do?siteGubun={}&menuGubun={}&bbsConfigFK={}&searchField=ALL&searchValue=&searchLowItem=ALL&currentPage=1&pkid={}",
                    site_gubun, menu_gubun, bbs_config_fk, pkid
                )
            } else {
                format!("javascript:{onclick}") // 폴백
            }
        } else if let Some(href) = a.as_ref().and_then(|n| n.attr("href")) {
            if href.starts_with("http") {
                href.to_string()
            } else {
                // 혹시 a에 href가 직접 들어있다면 동일한 컨텍스트 파라미터를 보강
                format!(
                    "https://www.swu.ac.kr{}&siteGubun={}&menuGubun={}&bbsConfigFK={}&searchField=ALL&searchValue=&searchLowItem=ALL&currentPage=1",
                    href, site_gubun, menu_gubun, bbs_config_fk
                )
            }
        } else {
            continue;
        };

        if !title.is_empty() {
            notices.push(Notice {
                title,
                date,
                url: detail_url,
            });
        }
    }

    if notices.is_empty() {
        eprintln!("[SWU] parsed 0 notices — 페이지 구조를 다시 확인하세요.");
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
                .link(n.url.clone()) // 날짜는 main에서 RFC2822로 정규화
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

/* ─── 유틸 ─── */

fn parse_board_move_pkid(onclick: &str) -> Option<String> {
    // 예: "boardMove('/front/boardview.do','506633')"
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
