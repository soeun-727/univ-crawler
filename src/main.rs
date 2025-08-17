mod crawler;
mod schools;
mod storage;

use schools::{sookmyung, dongduk, seoul};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 숙명
    let sm = sookmyung::fetch_notices()?;
    println!("<<숙명여자대학교 공지사항>>");
    for n in &sm { println!("{} [{}] ({})", n.title, n.date, n.url); }
    let sm_rss = sookmyung::create_rss(&sm);
    storage::save_rss_xml(&sm_rss, "school-rss/sookmyung/rss.xml")?;
    storage::save_markdown(&sm, "school-rss/sookmyung/index.md", "숙명여자대학교 학사 공지")?;

    // 동덕
    let dd = dongduk::fetch_notices()?;
    println!("\n<<동덕여자대학교 공지사항>>");
    for n in &dd { println!("{} [{}] ({})", n.title, n.date, n.url); }
    let dd_rss = dongduk::create_rss(&dd);
    storage::save_rss_xml(&dd_rss, "school-rss/dongduk/rss.xml")?;
    storage::save_markdown(&dd, "school-rss/dongduk/index.md", "동덕여자대학교 학사 공지")?;

    // 서울여대
    let sw = seoul::fetch_notices()?;
    println!("\n<<서울여자대학교 공지사항>>");
    for n in &sw { println!("{} [{}] ({})", n.title, n.date, n.url); }
    let sw_rss = seoul::create_rss(&sw);
    storage::save_rss_xml(&sw_rss, "school-rss/seoul/rss.xml")?;
    storage::save_markdown(&sw, "school-rss/seoul/index.md", "서울여자대학교 학사 공지")?;

    Ok(())
}