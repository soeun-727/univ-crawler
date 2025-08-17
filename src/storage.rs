use std::fs;
use std::path::Path;

pub fn save_rss_xml(channel: &rss::Channel, path: &str) -> std::io::Result<()> {
    let xml = channel.to_string();

    // 상위 디렉터리 자동 생성
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, xml)
}