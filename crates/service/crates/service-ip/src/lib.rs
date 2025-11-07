use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct IpApiResponse {
    #[serde(rename = "countryCode")]
    country_code: String,
}

#[derive(Debug, Deserialize)]
struct IpInfoResponse {
    country: String,
}

pub enum Region {
    CN,
    WORLD,
}

pub async fn get_user_region() -> Result<Region, Box<dyn std::error::Error>> {
    // 尝试第一个 API
    if let Ok(response) = reqwest::get("http://ip-api.com/json/").await {
        if let Ok(data) = response.json::<IpApiResponse>().await {
            return Ok(if data.country_code == "CN" {
                Region::CN
            } else {
                Region::WORLD
            });
        }
    }

    // 第一个 API 失败，尝试第二个
    let response = reqwest::get("https://ipinfo.io/json").await?;
    let data = response.json::<IpInfoResponse>().await?;

    Ok(if data.country != "CN" {
        Region::WORLD
    } else {
        Region::CN
    })
}
