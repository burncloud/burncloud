use anyhow::Result;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct AwsConfig {
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
}

impl AwsConfig {
    pub fn from_colon_string(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() < 3 {
            return Err(anyhow::anyhow!("Invalid AWS credential string. Expected: ACCESS_KEY:SECRET_KEY:REGION"));
        }
        Ok(Self {
            access_key: parts[0].to_string(),
            secret_key: parts[1].to_string(),
            region: parts[2].to_string(),
        })
    }
}

// Minimal percent encoding for AWS Path
// Encodes everything except unreserved characters: A-Z, a-z, 0-9, -, ., _, ~
// Also, '/' is preserved in the path.
fn aws_uri_encode(s: &str, encode_slash: bool) -> String {
    let mut encoded = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '~' {
            encoded.push(c);
        } else if c == '/' && !encode_slash {
            encoded.push(c);
        } else {
            // Percent encode
            let mut buf = [0; 4];
            let bytes = c.encode_utf8(&mut buf);
            for b in bytes.as_bytes() {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

pub fn sign_request(
    request: &mut reqwest::Request,
    config: &AwsConfig,
    body_bytes: &[u8],
) -> Result<()> {
    let now = Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();
    let service = "bedrock";

    // 1. Add required headers
    request.headers_mut().insert("x-amz-date", amz_date.parse()?);
    
    let host = request.url().host_str().unwrap_or_default().to_string();
    if !request.headers().contains_key("host") {
        request.headers_mut().insert("host", host.parse()?);
    }

    // 2. Canonical Request
    let method = request.method().as_str();
    let uri = request.url().path();
    
    // AWS requires normalized URI. Empty path is "/".
    // Importantly: Path segments must be URI encoded.
    // reqwest::Url::path() returns decoded path segments joined by /.
    // We need to encode them.
    
    let canonical_uri = if uri.is_empty() {
        "/".to_string()
    } else {
        // Split by '/' and encode each segment, then join.
        // But we must preserve leading/trailing slashes if they exist (though path() usually normalized).
        // Simpler: encode the whole path, but preserve '/'.
        aws_uri_encode(uri, false)
    };
    
    let query = request.url().query().unwrap_or("");
    let mut query_pairs: Vec<&str> = query.split('&').filter(|s| !s.is_empty()).collect();
    query_pairs.sort();
    // Query param names and values must also be encoded, but assume reqwest gives us encoded query string?
    // Actually reqwest::Url::query() returns the raw query string (encoded).
    // But AWS requires sorting by byte value. 
    // If we just split '&', we might split inside a value if it's not encoded? No, '&' is separator.
    // For safety, we should parse and re-encode, but for simple cases (chat), query is usually empty or simple.
    // Let's stick to simple sort for now.
    let canonical_querystring = query_pairs.join("&");

    // Canonical Headers
    let mut headers_to_sign = BTreeMap::new();
    for (k, v) in request.headers() {
        let key = k.as_str().to_lowercase();
        if key == "x-amz-date" || key == "host" || key == "content-type" { 
             if let Ok(val) = v.to_str() {
                 // Trim whitespace
                 let trim_val = val.trim();
                 // Compress multiple spaces? AWS spec says trim leading/trailing and convert sequential spaces to single space.
                 // For now, simple trim is usually enough.
                 headers_to_sign.insert(key, trim_val.to_string());
             }
        }
    }
    
    if !headers_to_sign.contains_key("host") {
        headers_to_sign.insert("host".to_string(), host.to_string());
    }
    if !headers_to_sign.contains_key("x-amz-date") {
        headers_to_sign.insert("x-amz-date".to_string(), amz_date.clone());
    }

    let mut canonical_headers = String::new();
    let mut signed_headers = String::new();
    for (key, value) in &headers_to_sign {
        canonical_headers.push_str(&format!("{}:{}\n", key, value));
        if !signed_headers.is_empty() {
            signed_headers.push(';');
        }
        signed_headers.push_str(key);
    }

    let payload_hash = hex::encode(Sha256::digest(body_bytes));

    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        method,
        canonical_uri,
        canonical_querystring,
        canonical_headers,
        signed_headers,
        payload_hash
    );

    // 3. String to Sign
    let algorithm = "AWS4-HMAC-SHA256";
    let credential_scope = format!("{}/{}/{}/aws4_request", date_stamp, config.region, service);
    let string_to_sign = format!(
        "{}\n{}\n{}\n{}",
        algorithm,
        amz_date,
        credential_scope,
        hex::encode(Sha256::digest(canonical_request.as_bytes()))
    );

    // 4. Calculate Signature
    let k_date = hmac_sha256(format!("AWS4{}", config.secret_key).as_bytes(), date_stamp.as_bytes())?;
    let k_region = hmac_sha256(&k_date, config.region.as_bytes())?;
    let k_service = hmac_sha256(&k_region, service.as_bytes())?;
    let k_signing = hmac_sha256(&k_service, b"aws4_request")?;
    let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes())?);

    // 5. Add Authorization Header
    let authorization_header = format!(
        "{} Credential={}/{}, SignedHeaders={}, Signature={}",
        algorithm,
        config.access_key,
        credential_scope,
        signed_headers,
        signature
    );

    request.headers_mut().insert("authorization", authorization_header.parse()?);

    Ok(())
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(key)?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

mod tests;