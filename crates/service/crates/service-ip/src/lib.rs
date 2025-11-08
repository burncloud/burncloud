use serde::Deserialize;
use burncloud_service_setting::SettingService;

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

impl Region {
    pub fn as_str(&self) -> &str {
        match self {
            Region::CN => "CN",
            Region::WORLD => "WORLD",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "CN" => Some(Region::CN),
            "WORLD" => Some(Region::WORLD),
            _ => None,
        }
    }
}

/// 获取用户地区（带缓存）
pub async fn get_location() -> Result<String, Box<dyn std::error::Error>> {
    let service = SettingService::new().await?;

    // 先查询缓存
    if let Some(location) = service.get("location").await? {
        return Ok(location);
    }

    // 没有缓存，查询地区
    let region = get_user_region().await?;
    let location = region.as_str().to_string();

    // 保存到数据库
    service.set("location", &location).await?;

    Ok(location)
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
