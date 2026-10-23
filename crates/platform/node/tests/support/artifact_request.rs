use super::super::{ArtifactSource, Digest};
use super::ArtifactRequest;

const SHA256: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn stores_only_a_valid_source_and_required_digest() {
    let source =
        ArtifactSource::parse("https://modelscope.cn/models/example/resolve/main/model.gguf")
            .expect("source should parse");
    let digest = Digest::parse(SHA256).expect("digest should parse");
    let request = ArtifactRequest::new(source.clone(), digest.clone());

    assert_eq!(request.source(), &source);
    assert_eq!(request.expected_digest(), &digest);
}
