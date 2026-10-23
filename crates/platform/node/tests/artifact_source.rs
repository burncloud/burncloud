use std::str::FromStr;

use burncloud_node_runtime::{ArtifactSource, ArtifactSourceError};
use url::Url;

const SUPPORTED_HOSTS: [&str; 3] = ["modelscope.cn", "hf-mirror.com", "huggingface.com"];

#[test]
fn parses_http_and_https_urls_for_each_supported_host() {
    for host in SUPPORTED_HOSTS {
        for scheme in ["http", "https"] {
            let source = ArtifactSource::parse(format!(
                "{scheme}://{host}/models/example/resolve/main/model.gguf"
            ))
            .expect("supported URL should parse");

            assert!(source.is_http());
            assert_eq!(source.as_http().host_str(), Some(host));
        }
    }
}

#[test]
fn accepts_case_insensitive_schemes_but_preserves_url_semantics() {
    let source =
        ArtifactSource::parse("HTTPS://huggingface.com/models/example/resolve/main/model.gguf")
            .expect("HTTPS URL should parse");

    assert_eq!(source.as_http().scheme(), "https");
}

#[test]
fn rejects_empty_invalid_scheme_and_unsupported_host_inputs() {
    assert_eq!(
        ArtifactSource::parse("   "),
        Err(ArtifactSourceError::Empty)
    );
    assert!(matches!(
        ArtifactSource::parse("https://"),
        Err(ArtifactSourceError::InvalidUrl(_))
    ));
    assert!(matches!(
        ArtifactSource::parse("ftp://huggingface.com/model.gguf"),
        Err(ArtifactSourceError::UnsupportedScheme(_))
    ));
    assert_eq!(
        ArtifactSource::parse("https://example.com/model.gguf"),
        Err(ArtifactSourceError::UnsupportedHost("example.com".into()))
    );
}

#[test]
fn rejects_local_path_and_file_url_inputs() {
    assert!(matches!(
        ArtifactSource::parse("models/model.gguf"),
        Err(ArtifactSourceError::InvalidUrl(_))
    ));
    assert!(matches!(
        ArtifactSource::parse("file:///models/model.gguf"),
        Err(ArtifactSourceError::UnsupportedScheme(_))
    ));
}

#[test]
fn url_conversion_and_from_str_share_the_same_validation() {
    let url = Url::parse("https://modelscope.cn/models/example/resolve/main/model.gguf")
        .expect("URL should parse");
    let from_url = ArtifactSource::try_from(&url).expect("supported URL should convert");
    let from_str = ArtifactSource::from_str(url.as_str()).expect("supported URL should parse");

    assert_eq!(from_url, from_str);
}
