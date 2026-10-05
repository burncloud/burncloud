use burncloud_service_setting::{SettingDatabase, SettingService};
use serde::Deserialize;

/// The first provider's response shape (`ip-api.com`): the country is `countryCode`.
#[derive(Debug, Deserialize)]
struct IpApiResponse {
    #[serde(rename = "countryCode")]
    country_code: String,
}

/// The second provider's response shape (`ipinfo.io`): the country is `country`.
#[derive(Debug, Deserialize)]
struct IpInfoResponse {
    country: String,
}

use std::str::FromStr;

/// The provider whose response uses `countryCode`.
pub const PRIMARY_PROVIDER_URL: &str = "http://ip-api.com/json/";
/// The provider whose response uses `country`.
pub const FALLBACK_PROVIDER_URL: &str = "https://ipinfo.io/json";

/// The settings key the resolved location is cached under.
pub const LOCATION_CACHE_KEY: &str = "location";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

impl FromStr for Region {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CN" => Ok(Region::CN),
            "WORLD" => Ok(Region::WORLD),
            _ => Err(()),
        }
    }
}

/// Map a provider's country field to a [`Region`].
///
/// **Extracted so the decision is testable without a network**, and because the two providers spell the same
/// field differently (`countryCode` and `country`) while the decision is identical. The rule is
/// "`CN` means China, anything else means the world", including an empty string -- which is what a provider
/// sending `"countryCode": ""` would produce, and which reads as WORLD rather than as a failure.
///
/// The comparison is **exact and case-sensitive**: `"cn"` and `" CN"` are not China by this rule. That is the
/// behaviour the code had; it is pinned here rather than changed, because whether a provider is trusted to
/// send a normalised code is a question for whoever owns this integration.
pub fn region_from_country_code(country_code: &str) -> Region {
    if country_code == "CN" {
        Region::CN
    } else {
        Region::WORLD
    }
}

/// Parse a provider body into a country code, choosing the shape from the URL.
///
/// The URL decides which field to read, because the two providers genuinely differ: reading `country` from
/// `ip-api`'s body would fail, and reading `countryCode` from `ipinfo`'s would too. A missing or mistyped field
/// is an error rather than a default -- see the tests, which pin that a body without the field fails rather than
/// being reported as WORLD.
fn country_code_from_body(url: &str, body: &str) -> Result<String, serde_json::Error> {
    if url.contains("ip-api") {
        serde_json::from_str::<IpApiResponse>(body).map(|r| r.country_code)
    } else {
        serde_json::from_str::<IpInfoResponse>(body).map(|r| r.country)
    }
}

/// 获取用户地区（带缓存）
pub async fn get_location() -> Result<String, Box<dyn std::error::Error>> {
    let db = SettingDatabase::new().await?;
    get_location_with(&db, PRIMARY_PROVIDER_URL, FALLBACK_PROVIDER_URL).await
}

/// Cached lookup against a database the caller owns, with the provider URLs injectable.
///
/// This is the seam the plan asks for ("先规划最小可测试接缝，不靠真实公网写单元测试"): the public
/// [`get_location`] keeps its signature and behaviour -- it creates its own database and uses the real URLs --
/// while the decisions live here, where a test can point them at a local server and a temporary database.
pub async fn get_location_with(
    db: &SettingDatabase,
    primary_url: &str,
    fallback_url: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // 先查询缓存
    if let Some(location) = SettingService::get(db, LOCATION_CACHE_KEY).await? {
        return Ok(location);
    }

    // 没有缓存，查询地区
    let region = get_user_region_with(primary_url, fallback_url).await?;
    let location = region.as_str().to_string();

    // 保存到数据库
    SettingService::set(db, LOCATION_CACHE_KEY, &location).await?;

    Ok(location)
}

pub async fn get_user_region() -> Result<Region, Box<dyn std::error::Error>> {
    get_user_region_with(PRIMARY_PROVIDER_URL, FALLBACK_PROVIDER_URL).await
}

/// Resolve the region from two providers, with both URLs injectable.
///
/// The primary is tried first and the fallback only if it fails **to produce a usable country code** -- a
/// transport error, a non-success status, a body that does not parse, or a body without the field. The second
/// provider has **no further fallback**: if it fails, this returns an error rather than guessing WORLD, so a
/// caller can tell "the world" from "could not find out".
pub async fn get_user_region_with(
    primary_url: &str,
    fallback_url: &str,
) -> Result<Region, Box<dyn std::error::Error>> {
    // 尝试第一个 API
    if let Ok(response) = reqwest::get(primary_url).await {
        if let Ok(text) = response.text().await {
            if let Ok(code) = country_code_from_body(primary_url, &text) {
                return Ok(region_from_country_code(&code));
            }
        }
    }

    // 第一个 API 失败，尝试第二个
    let response = reqwest::get(fallback_url).await?;
    let text = response.text().await?;
    let data = country_code_from_body(fallback_url, &text)?;

    Ok(region_from_country_code(&data))
}
