use playwright_rs::protocol::{Playwright, Viewport};
use std::{env, error::Error, fs, path::PathBuf, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let route = env::args().nth(1).ok_or("expected route argument")?;
    if !route.starts_with('/') || route.contains("..") || route.contains('?') || route.contains('#') {
        return Err("invalid local route".into());
    }

    let output = PathBuf::from("screenshots").join(format!(
        "{}.png",
        route.trim_matches('/').replace('/', "-")
    ));
    fs::create_dir_all("screenshots")?;

    playwright_rs::install_browsers(Some(&["chromium"])).await?;
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;
    page.set_viewport_size(Viewport { width: 1440, height: 900 }).await?;
    let url = format!("http://127.0.0.1:8080{route}");
    page.goto(&url, None).await?;
    tokio::time::sleep(Duration::from_secs(3)).await?;

    let png = page.screenshot(None).await?;
    if png.len() < 1024 {
        return Err(format!("screenshot is unexpectedly small for {url}").into());
    }
    fs::write(&output, png)?;
    println!("Captured {} => {}", route, output.display());
    browser.close().await?;
    Ok(())
}
