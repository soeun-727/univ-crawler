// src/crawler.rs
use crate::schools::{sookmyung, dongduk, seoul}; // ← 추가

pub fn crawl(school_name: Option<&str>) -> Result<Vec<sookmyung::Notice>, Box<dyn std::error::Error>> {
    match school_name {
        Some("sookmyung") => {
            println!("<<숙명여자대학교 공지사항>>");
            let notices = sookmyung::fetch_notices()?;
            Ok(notices)
        }
        Some("dongduk") => {
            println!("<<동덕여자대학교 공지사항>>");
            let notices = dongduk::fetch_notices()?;
            Ok(notices)
        }
        Some("seoul") => {
            println!("<<서울여자대학교 공지사항>>");
            let notices = seoul::fetch_notices()?;
            Ok(notices)
        }
        Some(other) => {
            println!("학교 '{}'는 없습니다.", other);
            Ok(Vec::new())
        }
        None => {
            println!("<<전체 학교 공지사항>>");
            let mut notices = Vec::new();                   // ← 변경
            notices.extend(sookmyung::fetch_notices()?);           // ← 변경
            notices.extend(dongduk::fetch_notices()?);             // ← 변경
            notices.extend(seoul::fetch_notices()?);               // ← 변경
            Ok(notices)
        }
    }
}
