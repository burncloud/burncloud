#[cfg(test)]
mod tests {
    use crate::{aws_uri_encode, AwsConfig}; // Absolute path within crate

    #[test]
    fn test_aws_config_parsing() {
        let config = AwsConfig::from_colon_string("AK:SK:us-east-1").unwrap();
        assert_eq!(config.access_key, "AK");
        assert_eq!(config.secret_key, "SK");
        assert_eq!(config.region, "us-east-1");

        assert!(AwsConfig::from_colon_string("InvalidString").is_err());
    }

    #[test]
    fn test_uri_encode() {
        // Basic chars
        assert_eq!(aws_uri_encode("abc-123_.~", false), "abc-123_.~");
        // Space
        assert_eq!(aws_uri_encode("hello world", false), "hello%20world");
        // Slash preserved
        assert_eq!(
            aws_uri_encode("/path/to/resource", false),
            "/path/to/resource"
        );
        // Slash encoded
        assert_eq!(
            aws_uri_encode("/path/to/resource", true),
            "%2Fpath%2Fto%2Fresource"
        );
        // Colon
        assert_eq!(aws_uri_encode("v1:0", false), "v1%3A0");
    }
}
