use burncloud_node_runtime::{Digest, DigestAlgorithm, DigestError};

const MD5: &str = "0123456789ABCDEF0123456789ABCDEF";
const SHA1: &str = "0123456789ABCDEF0123456789ABCDEF01234567";
const SHA256: &str = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";
const SHA384: &str =
    "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";
const SHA512: &str =
    "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";

#[test]
fn exposes_digest_and_hex_lengths_for_all_supported_algorithms() {
    let expected = [
        (DigestAlgorithm::Md5, 16, 32),
        (DigestAlgorithm::Sha1, 20, 40),
        (DigestAlgorithm::Sha256, 32, 64),
        (DigestAlgorithm::Sha384, 48, 96),
        (DigestAlgorithm::Sha512, 64, 128),
    ];

    for (algorithm, digest_bytes, hex_length) in expected {
        assert_eq!(algorithm.digest_bytes(), digest_bytes);
        assert_eq!(algorithm.expected_hex_length(), hex_length);
    }
}

#[test]
fn parses_algorithm_aliases_and_displays_canonical_names() {
    for alias in ["sha256", "SHA256", "sha-256", "SHA-256"] {
        assert_eq!(alias.parse(), Ok(DigestAlgorithm::Sha256));
    }
    for alias in ["sha1", "SHA1", "sha-1", "SHA-1"] {
        assert_eq!(alias.parse(), Ok(DigestAlgorithm::Sha1));
    }
    for alias in ["sha384", "SHA384", "sha-384", "SHA-384"] {
        assert_eq!(alias.parse(), Ok(DigestAlgorithm::Sha384));
    }

    assert_eq!(DigestAlgorithm::Md5.to_string(), "md5");
    assert_eq!(DigestAlgorithm::Sha1.to_string(), "sha1");
    assert_eq!(DigestAlgorithm::Sha256.to_string(), "sha256");
    assert_eq!(DigestAlgorithm::Sha384.to_string(), "sha384");
    assert_eq!(DigestAlgorithm::Sha512.to_string(), "sha512");
}

#[test]
fn parses_and_normalizes_all_supported_digest_values() {
    let cases = [
        ("md5", MD5, DigestAlgorithm::Md5),
        ("sha1", SHA1, DigestAlgorithm::Sha1),
        ("sha256", SHA256, DigestAlgorithm::Sha256),
        ("sha384", SHA384, DigestAlgorithm::Sha384),
        ("sha512", SHA512, DigestAlgorithm::Sha512),
    ];

    for (name, value, algorithm) in cases {
        let digest = Digest::parse(&format!("{name}:{value}")).expect("digest should parse");
        assert_eq!(digest.algorithm(), algorithm);
        assert_eq!(digest.value(), value.to_ascii_lowercase());
        assert_eq!(
            digest.to_string(),
            format!("{name}:{}", value.to_ascii_lowercase())
        );
    }
}

#[test]
fn parses_through_from_str() {
    let digest: Digest = format!("sha256:{SHA256}")
        .parse()
        .expect("digest should parse");

    assert_eq!(digest.algorithm(), DigestAlgorithm::Sha256);
    assert_eq!(digest.value(), SHA256.to_ascii_lowercase());
}

#[test]
fn rejects_missing_algorithm_unknown_algorithm_wrong_length_and_non_hex() {
    assert_eq!(Digest::parse("value"), Err(DigestError::MissingAlgorithm));
    assert_eq!(
        Digest::parse("sha3:0123"),
        Err(DigestError::UnsupportedAlgorithm("sha3".into()))
    );
    assert_eq!(
        Digest::new(DigestAlgorithm::Sha256, "0123"),
        Err(DigestError::BadLength {
            expected: 64,
            actual: 4,
        })
    );
    assert_eq!(
        Digest::new(DigestAlgorithm::Sha256, "z".repeat(64)),
        Err(DigestError::NotHex)
    );
}
