mod crawler;
mod schools;
mod storage;

use schools::sookmyung::{fetch_notices, create_rss};
//여기 위에 처럼 dongduk이랑 seoul 불러와서 use 할 수 있게 수정 부탁드립니다!
//형식이 다를 거 같아서 제가 못해서 요청드려요

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let notices = fetch_notices()?;  

    for notice in &notices {
        println!("{}, [{}]", notice.title, notice.date);
    }

    let rss_channel = create_rss(&notices);
    storage::save_rss_xml(&rss_channel, "school-rss/swu/rss.xml")?;

    Ok(())
}
