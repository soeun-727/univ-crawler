//src/crawler.rs
use crate::schools::sookmyung;

//여기는 sookmyung.rs랑은 호환이 되는데
//seoul.rs랑 dongduk.rs 경우 notices -> fetch_notices로 안돼있는 경우 아래 식이 작동을 안할 거 같아서
//아래 코드랑 같이 작동하는지 각 학교 코드확인해주시고 안되면은 각 학교 코드 수정해주시면 될 것 같습니다! 

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
            let notices = sookmyung::fetch_notices()?;
            let notices = dongduk::fetch_notices()?;
            let notices = seoul::fetch_notices()?;
            Ok(notices)
        }
    }
}
