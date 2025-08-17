use std::fs;
use std::path::Path;

pub fn save_rss_xml(channel: &rss::Channel, path: &str) -> std::io::Result<()> {
    let xml = channel.to_string();
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, xml)
}

// notices를 마크다운 리스트로 저장
pub fn save_markdown(notices: &[crate::schools::sookmyung::Notice], path: &str, title: &str) -> std::io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    let mut md = String::new();
    md.push_str(&format!("# {title}\n\n"));
    for n in notices {
        md.push_str(&format!("- [{}]({}) — `{}`\n", n.title, n.url, n.date));
    }
    fs::write(path, md)
}