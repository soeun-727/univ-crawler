use std::fs;

pub fn save_rss_xml(channel: &rss::Channel, path: &str) -> std::io::Result<()> {
    let xml = channel.to_string();
    fs::write(path, xml)
}
