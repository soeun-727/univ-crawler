// src/main.rs
mod crawler;
mod schools;
mod storage;

use schools::{sookmyung, dongduk, seoul};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::io::{Error as IoError, ErrorKind};

// 날짜/URL 정규화에 필요한 크레이트
use chrono::{Local, NaiveDate, Datelike, TimeZone};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 시작 시 한 번 파일 생성(블로킹) → 전용 스레드에서 안전하게 실행
    if let Err(e) = tokio::task::spawn_blocking(|| run_once_generate_files())
        .await
        .expect("spawn_blocking failed")
    {
        eprintln!("초기 파일 생성 중 오류: {e}");
    }

    // HTTP 서버: 요청 시 실시간 크롤링 → RSS XML 반환
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

/* ───────────── RFC2822 날짜/절대 URL 정규화 ───────────── */

fn to_rfc2822(date_raw: &str) -> String {
    // 허용 포맷: "YYYY.MM.DD", "YYYY-MM-DD", "YYYY/MM/DD", 끝에 점(.) 허용
    let cleaned = date_raw.trim().trim_end_matches('.');
    let seps = ['.', '-', '/'];

    let parsed = seps.iter().find_map(|sep| {
        let parts: Vec<_> = cleaned.split(*sep).map(|s| s.trim()).collect();
        if parts.len() == 3 {
            let (y, m, d) = (parts[0], parts[1], parts[2]);
            if let (Ok(yy), Ok(mm), Ok(dd)) = (y.parse::<i32>(), m.parse::<u32>(), d.parse::<u32>()) {
                NaiveDate::from_ymd_opt(yy, mm, dd)
            } else { None }
        } else { None }
    });

    let dt = match parsed {
        Some(naive) => Local.with_ymd_and_hms(naive.year(), naive.month(), naive.day(), 0, 0, 0)
                            .single()
                            .unwrap_or_else(|| Local::now()),
        None => Local::now(),
    };

    dt.to_rfc2822()
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

// 모든 학교가 sookmyung::Notice 타입을 사용하므로, 그 타입으로 정규화
fn normalize_notices(school_key: &str, src: &[sookmyung::Notice]) -> Vec<sookmyung::Notice> {
    src.iter().map(|n| {
        sookmyung::Notice {
            title: n.title.clone(),
            date:  to_rfc2822(&n.date),
            url:   ensure_absolute_url(school_key, &n.url),
        }
    }).collect()
}

/* ───────────── 기존 파일 생성(정규화 적용) ───────────── */

fn run_once_generate_files() -> Result<(), IoError> {
    // 숙명
    let sm_raw = sookmyung::fetch_notices()
        .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
    let sm = normalize_notices("sookmyung", &sm_raw);
    println!("<<숙명여자대학교 공지사항>>");
    for n in &sm { println!("{} [{}] ({})", n.title, n.date, n.url); }
    let sm_rss = sookmyung::create_rss(&sm);
    storage::save_rss_xml(&sm_rss, "school-rss/sookmyung/rss.xml")?;
    storage::save_markdown(&sm, "school-rss/sookmyung/index.md", "숙명여자대학교 학사 공지")?;

    // 동덕
    let dd_raw = dongduk::fetch_notices()
        .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
    let dd = normalize_notices("dongduk", &dd_raw);
    println!("\n<<동덕여자대학교 공지사항>>");
    for n in &dd { println!("{} [{}] ({})", n.title, n.date, n.url); }
    let dd_rss = dongduk::create_rss(&dd);
    storage::save_rss_xml(&dd_rss, "school-rss/dongduk/rss.xml")?;
    storage::save_markdown(&dd, "school-rss/dongduk/index.md", "동덕여자대학교 학사 공지")?;

    // 서울여대
    let sw_raw = seoul::fetch_notices()
        .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
    let sw = normalize_notices("seoul", &sw_raw);
    println!("\n<<서울여자대학교 공지사항>>");
    for n in &sw { println!("{} [{}] ({})", n.title, n.date, n.url); }
    let sw_rss = seoul::create_rss(&sw);
    storage::save_rss_xml(&sw_rss, "school-rss/seoul/rss.xml")?;
    storage::save_markdown(&sw, "school-rss/seoul/index.md", "서울여자대학교 학사 공지")?;

    Ok(())
}

/* ───────────── HTTP 핸들러(정규화 적용) ───────────── */

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
