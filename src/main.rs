mod crawler;
mod schools;
mod storage;

use schools::sookmyung::{fetch_notices, create_rss};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let notices = fetch_notices()?;  

    for notice in &notices {
        println!("{}, [{}]", notice.title, notice.date);
    }

    let rss_channel = create_rss(&notices);
    storage::save_rss_xml(&rss_channel, "school-rss/swu/rss.xml")?;

    Ok(())
}
