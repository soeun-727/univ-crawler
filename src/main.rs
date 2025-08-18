// src/main.rs
mod crawler;
mod schools;
mod storage;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use schools::{dongduk, seoul, sookmyung};
use std::io::{Error as IoError, ErrorKind};

// 날짜/URL 정규화
use chrono::{Datelike, Local, NaiveDate, TimeZone};

const SITE_ROOT: &str = "public"; // GitHub Pages에 배포할 루트(빌드 산출물 저장 위치)

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // ── 원샷 모드: 파일만 만들고 종료 (액션/로컬용) ─────────────────────
    // 사용법: cargo run -- --oneshot  (또는 --one-shot)
    let args: Vec<String> = std::env::args().collect();
    let is_oneshot = args.iter().any(|a| a == "--oneshot" || a == "--one-shot");
    // GitHub Actions에서는 자동으로 원샷 처리
    let is_ci = std::env::var("GITHUB_ACTIONS").is_ok();

    if is_oneshot || is_ci {
        // ✅ 동기 크롤링을 별도 블로킹 스레드에서 실행 → 런타임 드롭 패닉 방지
        let res = tokio::task::spawn_blocking(|| run_once_generate_files())
            .await
            .expect("spawn_blocking failed");

        match res {
            Ok(()) => {
                println!("원샷 파일 생성 완료. 서버는 실행하지 않습니다.");
                return Ok(());
            }
            Err(e) => {
                eprintln!("원샷 파일 생성 실패: {e}");
                return Err(e);
            }
        }
    }

    // ── 서버 실행 전 1회 파일 생성(블로킹 스레드) ──────────────────────
    if let Err(e) = tokio::task::spawn_blocking(|| run_once_generate_files())
        .await
        .expect("spawn_blocking failed")
    {
        eprintln!("초기 파일 생성 중 오류: {e}");
    }

    // ── HTTP 서버: 요청 시 실시간 크롤링 → RSS XML 반환 ───────────────
    HttpServer::new(|| {
        App::new()
            .route("/healthz", web::get().to(|| async { "ok" }))
            // 예: /school-rss/sookmyung/rss.xml, /school-rss/seoul/rss.xml, /school-rss/dongduk/rss.xml
            .route("/school-rss/{school}/rss.xml", web::get().to(rss_endpoint))
    })
    .bind(("0.0.0.0", 8080))?
    .workers(2)
    .run()
    .await
}

/* ───────────── 날짜 추출/변환 & 절대 URL 정규화 ───────────── */

/// 문자열 토큰들에서 YYYY.MM.DD / YYYY-MM-DD / YYYY/MM/DD 패턴을 찾아냄
fn extract_date_token(s: &str) -> Option<(i32, u32, u32)> {
    let seps = ['.', '-', '/'];
    for token in s.split_whitespace() {
        let t = token.trim().trim_end_matches('.');
        for sep in seps {
            let parts: Vec<_> = t.split(sep).map(|x| x.trim()).collect();
            if parts.len() != 3 {
                continue;
            }
            let (y, m, d) = (parts[0], parts[1], parts[2]);
            if let (Ok(yy), Ok(mm), Ok(dd)) = (y.parse::<i32>(), m.parse::<u32>(), d.parse::<u32>()) {
                if NaiveDate::from_ymd_opt(yy, mm, dd).is_some() {
                    return Some((yy, mm, dd));
                }
            }
        }
    }
    None
}

/// pubDate용 RFC 2822 변환. (실패 시 현재 시각)
fn to_rfc2822(date_raw: &str) -> String {
    if let Some((yy, mm, dd)) = extract_date_token(date_raw) {
        if let Some(naive) = NaiveDate::from_ymd_opt(yy, mm, dd) {
            if let Some(dt) = Local
                .with_ymd_and_hms(naive.year(), naive.month(), naive.day(), 0, 0, 0)
                .single()
            {
                return dt.to_rfc2822();
            }
        }
    }
    Local::now().to_rfc2822()
}

fn ensure_absolute_url(school: &str, url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }
    match school {
        "sookmyung" | "sm" | "숙명" => format!("https://www.sookmyung.ac.kr{}", url),
        "seoul" | "swu" | "서울" => format!("https://www.swu.ac.kr{}", url),
        "dongduk" | "dd" | "동덕" => format!("https://www.dongduk.ac.kr{}", url),
        _ => url.to_string(),
    }
}

// sookmyung::Notice 형식으로 정규화
fn normalize_notices(school_key: &str, src: &[sookmyung::Notice]) -> Vec<sookmyung::Notice> {
    src.iter()
        .map(|n| sookmyung::Notice {
            title: n.title.clone(),
            date: to_rfc2822(&n.date),                    // 조회수 텍스트 섞여도 날짜만 추출
            url: ensure_absolute_url(school_key, &n.url), // 절대 URL 보장
        })
        .collect()
}

/* ───────────── 파일 생성(정규화 적용) ───────────── */

fn run_once_generate_files() -> Result<(), IoError> {
    // 숙명
    let sm_raw = sookmyung::fetch_notices()
        .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
    let sm = normalize_notices("sookmyung", &sm_raw);
    println!("<<숙명여자대학교 공지사항>>");
    for n in &sm {
        println!("{} [{}] ({})", n.title, n.date, n.url);
    }
    let sm_rss = sookmyung::create_rss(&sm);
    storage::save_rss_xml(&sm_rss, &format!("{}/school-rss/sookmyung/rss.xml", SITE_ROOT))?;
    storage::save_markdown(
        &sm,
        &format!("{}/school-rss/sookmyung/index.md", SITE_ROOT),
        "숙명여자대학교 학사 공지",
    )?;

    // 동덕
    let dd_raw = dongduk::fetch_notices()
        .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
    let dd = normalize_notices("dongduk", &dd_raw);
    println!("\n<<동덕여자대학교 공지사항>>");
    for n in &dd {
        println!("{} [{}] ({})", n.title, n.date, n.url);
    }
    let dd_rss = dongduk::create_rss(&dd);
    storage::save_rss_xml(&dd_rss, &format!("{}/school-rss/dongduk/rss.xml", SITE_ROOT))?;
    storage::save_markdown(
        &dd,
        &format!("{}/school-rss/dongduk/index.md", SITE_ROOT),
        "동덕여자대학교 학사 공지",
    )?;

    // 서울여대
    let sw_raw = seoul::fetch_notices()
        .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
    let sw = normalize_notices("seoul", &sw_raw);
    println!("\n<<서울여자대학교 공지사항>>");
    for n in &sw {
        println!("{} [{}] ({})", n.title, n.date, n.url);
    }
    let sw_rss = seoul::create_rss(&sw);
    storage::save_rss_xml(&sw_rss, &format!("{}/school-rss/seoul/rss.xml", SITE_ROOT))?;
    storage::save_markdown(
        &sw,
        &format!("{}/school-rss/seoul/index.md", SITE_ROOT),
        "서울여자대학교 학사 공지",
    )?;

    Ok(())
}

/* ───────────── HTTP 핸들러 ───────────── */

async fn rss_endpoint(path: web::Path<(String,)>) -> impl Responder {
    let school = path.into_inner().0.to_lowercase();
    let result = web::block(move || generate_rss_xml(&school)).await;

    match result {
        Ok(Ok(xml)) => HttpResponse::Ok()
            .content_type("application/rss+xml; charset=utf-8")
            .body(xml),
        Ok(Err(e)) => {
            eprintln!("generate_rss_xml error: {e}");
            HttpResponse::InternalServerError().body("failed to generate rss")
        }
        Err(e) => {
            eprintln!("web::block join error: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

fn generate_rss_xml(school: &str) -> Result<String, IoError> {
    match school {
        "sookmyung" | "sm" | "숙명" => {
            let items_raw = sookmyung::fetch_notices()
                .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
            let items = normalize_notices("sookmyung", &items_raw);
            Ok(sookmyung::create_rss(&items).to_string())
        }
        "seoul" | "swu" | "서울" => {
            let items_raw = seoul::fetch_notices()
                .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
            let items = normalize_notices("seoul", &items_raw);
            Ok(seoul::create_rss(&items).to_string())
        }
        "dongduk" | "dd" | "동덕" => {
            let items_raw = dongduk::fetch_notices()
                .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
            let items = normalize_notices("dongduk", &items_raw);
            Ok(dongduk::create_rss(&items).to_string())
        }
        other => Err(IoError::new(
            ErrorKind::InvalidInput,
            format!("unknown school: {other}"),
        )),
    }
}
