use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    univ_crawler::fetch_notices()?;
    Ok(())
}
