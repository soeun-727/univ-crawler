// src/main.rs
mod crawler;
mod schools;
mod storage;

use schools::{sookmyung, dongduk, seoul};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::io::{Error as IoError, ErrorKind};

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

// 기존 로직(파일 생성)을 함수로 분리
fn run_once_generate_files() -> Result<(), IoError> {
    // 숙명
    let sm = sookmyung::fetch_notices()
        .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
    println!("<<숙명여자대학교 공지사항>>");
    for n in &sm { println!("{} [{}] ({})", n.title, n.date, n.url); }
    let sm_rss = sookmyung::create_rss(&sm);
    storage::save_rss_xml(&sm_rss, "school-rss/sookmyung/rss.xml")?;
    storage::save_markdown(&sm, "school-rss/sookmyung/index.md", "숙명여자대학교 학사 공지")?;

    // 동덕
    let dd = dongduk::fetch_notices()
        .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
    println!("\n<<동덕여자대학교 공지사항>>");
    for n in &dd { println!("{} [{}] ({})", n.title, n.date, n.url); }
    let dd_rss = dongduk::create_rss(&dd);
    storage::save_rss_xml(&dd_rss, "school-rss/dongduk/rss.xml")?;
    storage::save_markdown(&dd, "school-rss/dongduk/index.md", "동덕여자대학교 학사 공지")?;

    // 서울여대
    let sw = seoul::fetch_notices()
        .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
    println!("\n<<서울여자대학교 공지사항>>");
    for n in &sw { println!("{} [{}] ({})", n.title, n.date, n.url); }
    let sw_rss = seoul::create_rss(&sw);
    storage::save_rss_xml(&sw_rss, "school-rss/seoul/rss.xml")?;
    storage::save_markdown(&sw, "school-rss/seoul/index.md", "서울여자대학교 학사 공지")?;

    Ok(())
}

// HTTP 핸들러: 요청 시 실시간 크롤링 → RSS XML 반환
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

// 학교별 실시간 RSS XML 생성
fn generate_rss_xml(school: &str) -> Result<String, IoError> {
    match school {
        "sookmyung" | "sm" | "숙명" => {
            let items = sookmyung::fetch_notices()
                .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
            let ch = sookmyung::create_rss(&items);
            Ok(ch.to_string())
        }
        "seoul" | "swu" | "서울" => {
            let items = seoul::fetch_notices()
                .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
            let ch = seoul::create_rss(&items);
            Ok(ch.to_string())
        }
        "dongduk" | "dd" | "동덕" => {
            let items = dongduk::fetch_notices()
                .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;
            let ch = dongduk::create_rss(&items);
            Ok(ch.to_string())
        }
        other => Err(IoError::new(
            ErrorKind::InvalidInput,
            format!("unknown school: {other}"),
        )),
    }
}
