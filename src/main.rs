mod crawler;
mod schools;
mod storage;

use schools::{sookmyung, dongduk, seoul}; // ← 추가

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 숙명여대
    let smu = sookmyung::fetch_notices()?;
    println!("<<숙명여자대학교 공지사항>>");
    for n in &smu {
        println!("{} [{}] ({})", n.title, n.date, n.url);
    }
    let smu_rss = sookmyung::create_rss(&smu);
    storage::save_rss_xml(&smu_rss, "school-rss/swu/rss.xml")?; // 기존 경로 유지

    // 동덕여대
    let ddu = dongduk::fetch_notices()?;
    println!("\n<<동덕여자대학교 공지사항>>");
    for n in &ddu {
        println!("{} [{}] ({})", n.title, n.date, n.url);
    }
    // 필요하면 아래 두 줄 주석 해제해서 RSS도 저장 가능
    // let ddu_rss = dongduk::create_rss(&ddu);
    // storage::save_rss_xml(&ddu_rss, "school-rss/dongduk/rss.xml")?;

    // 서울여대
    let swu = seoul::fetch_notices()?;
    println!("\n<<서울여자대학교 공지사항>>");
    for n in &swu {
        println!("{} [{}] ({})", n.title, n.date, n.url);
    }
    // 필요하면 아래 두 줄 주석 해제해서 RSS도 저장 가능
    // let swu_rss = seoul::create_rss(&swu);
    // storage::save_rss_xml(&swu_rss, "school-rss/seoul/rss.xml")?;

    Ok(())
}
