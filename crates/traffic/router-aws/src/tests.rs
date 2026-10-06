#[cfg(test)]
#[allow(
    clippy::module_inception,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "unit-test module: the module is named `tests` like its parent file, and assertions on known-good fixtures fail fast with unwrap/expect"
)]
mod tests {
    use crate::{aws_uri_encode, sign_request_at, AwsConfig};
    use chrono::{TimeZone, Utc};

    /// A fixed instant, so every signature below is reproducible.
    ///
    /// `sign_request_at` exists for this: a SigV4 signature is a function of the timestamp through
    /// `x-amz-date` and the credential scope, so without a supplied clock there is nothing to compare against.
    fn fixed_clock() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap()
    }

    /// The date and scope the fixed clock produces, used to assert the parts of the header that are not the
    /// signature.
    const EXPECTED_AMZ_DATE: &str = "20260102T030405Z";
    const EXPECTED_DATE_STAMP: &str = "20260102";

    fn config() -> AwsConfig {
        AwsConfig {
            access_key: "AKIDEXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".to_string(),
            region: "us-east-1".to_string(),
        }
    }

    /// Build a request, sign it at the fixed clock, and return the `Authorization` header.
    fn authorization_for(method: &str, url: &str, headers: &[(&str, &str)], body: &str) -> String {
        let mut request = reqwest::Client::new()
            .request(method.parse().unwrap(), url)
            .build()
            .unwrap_or_else(|e| panic!("building {method} {url}: {e}"));
        for (k, v) in headers {
            // Explicit types: `parse()` on a `&str` is generic over the target, and the header map's `insert`
            // accepts either a name or a tuple, so inference has nothing to go on.
            let name: reqwest::header::HeaderName = (*k).parse().unwrap();
            let value: reqwest::header::HeaderValue = (*v).parse().unwrap();
            request.headers_mut().insert(name, value);
        }
        sign_request_at(&mut request, &config(), body.as_bytes(), fixed_clock())
            .unwrap_or_else(|e| panic!("signing: {e}"));
        request
            .headers()
            .get("authorization")
            .expect("the authorization header is always added")
            .to_str()
            .unwrap()
            .to_string()
    }

    /// Just the hex signature, which is the part that must change when the input does.
    fn signature_of(authorization: &str) -> &str {
        authorization
            .split("Signature=")
            .nth(1)
            .expect("the header carries a Signature=")
    }

    // ---------------------------------------------------------------------------------------
    // the fixture itself
    // ---------------------------------------------------------------------------------------

    #[test]
    fn a_signed_request_carries_the_fixed_date_and_a_well_formed_authorization_header() {
        // The control. Without it, a test asserting "the signature changed" would pass if signing were
        // producing garbage, and one asserting "the signature is stable" would pass if signing did nothing.
        let auth = authorization_for("GET", "https://example.amazonaws.com/", &[], "");

        println!("authorization: {auth}");

        assert!(
            auth.starts_with("AWS4-HMAC-SHA256 "),
            "the algorithm is named first: {auth}"
        );
        assert!(
            auth.contains(&format!(
                "Credential=AKIDEXAMPLE/{EXPECTED_DATE_STAMP}/us-east-1/bedrock/aws4_request"
            )),
            "the credential scope is access key, date stamp, region and service: {auth}"
        );
        assert!(
            auth.contains("SignedHeaders="),
            "the signed header list is present: {auth}"
        );
        assert_eq!(
            signature_of(&auth).len(),
            64,
            "a SHA-256 HMAC in hex is 64 characters"
        );
        assert!(
            signature_of(&auth).chars().all(|c| c.is_ascii_hexdigit()),
            "and it is hex: {}",
            signature_of(&auth)
        );
    }

    #[test]
    fn the_x_amz_date_header_is_derived_from_the_supplied_clock() {
        // The half of the timestamp that is visible: the header itself.
        let mut request = reqwest::Client::new()
            .get("https://example.amazonaws.com/")
            .build()
            .unwrap();
        sign_request_at(&mut request, &config(), b"", fixed_clock()).unwrap();

        let amz_date = request
            .headers()
            .get("x-amz-date")
            .expect("x-amz-date is added")
            .to_str()
            .unwrap();
        assert_eq!(
            amz_date, EXPECTED_AMZ_DATE,
            "the clock reading, formatted for AWS"
        );

        assert_eq!(
            request
                .headers()
                .get("host")
                .expect("host is added when absent")
                .to_str()
                .unwrap(),
            "example.amazonaws.com"
        );
    }

    // ---------------------------------------------------------------------------------------
    // determinism, and every input reaching the signature
    // ---------------------------------------------------------------------------------------

    #[test]
    fn the_same_inputs_produce_the_same_signature() {
        // SigV4 is a pure function of its inputs, so two calls must agree. A signature that varied would make
        // every downstream assertion meaningless, so this is checked before any of them.
        let first = authorization_for(
            "POST",
            "https://example.amazonaws.com/model/x/invoke",
            &[],
            "{\"a\":1}",
        );
        let second = authorization_for(
            "POST",
            "https://example.amazonaws.com/model/x/invoke",
            &[],
            "{\"a\":1}",
        );
        assert_eq!(
            first, second,
            "the same inputs must produce the same header"
        );
    }

    #[test]
    fn every_part_of_the_request_reaches_the_signature() {
        // The plan asks for "HTTP method/path/query/header/body 变化导致预期签名变化". All five are varied here
        // from one base, and each must produce a different signature -- otherwise that part is not signed and
        // an attacker could change it without invalidating the request.
        let base = authorization_for(
            "POST",
            "https://example.amazonaws.com/model/anthropic.claude/invoke",
            &[("content-type", "application/json")],
            "{\"prompt\":\"hello\"}",
        );

        let cases: Vec<(&str, String)> = vec![
            (
                "method",
                authorization_for(
                    "PUT",
                    "https://example.amazonaws.com/model/anthropic.claude/invoke",
                    &[("content-type", "application/json")],
                    "{\"prompt\":\"hello\"}",
                ),
            ),
            (
                "path",
                authorization_for(
                    "POST",
                    "https://example.amazonaws.com/model/other.claude/invoke",
                    &[("content-type", "application/json")],
                    "{\"prompt\":\"hello\"}",
                ),
            ),
            (
                "query",
                authorization_for(
                    "POST",
                    "https://example.amazonaws.com/model/anthropic.claude/invoke?version=2",
                    &[("content-type", "application/json")],
                    "{\"prompt\":\"hello\"}",
                ),
            ),
            (
                "a signed header",
                authorization_for(
                    "POST",
                    "https://example.amazonaws.com/model/anthropic.claude/invoke",
                    &[("content-type", "text/plain")],
                    "{\"prompt\":\"hello\"}",
                ),
            ),
            (
                "body",
                authorization_for(
                    "POST",
                    "https://example.amazonaws.com/model/anthropic.claude/invoke",
                    &[("content-type", "application/json")],
                    "{\"prompt\":\"goodbye\"}",
                ),
            ),
        ];

        let base_sig = signature_of(&base).to_string();
        for (what, auth) in &cases {
            let sig = signature_of(auth);
            println!("{what}: {sig}");
            assert_ne!(
                sig, base_sig,
                "changing the {what} must change the signature; if it does not, that part of the request is \
                 not covered by the signature"
            );
        }

        // And the base is stable, so the five differences above are the changes and not drift.
        assert_eq!(
            authorization_for(
                "POST",
                "https://example.amazonaws.com/model/anthropic.claude/invoke",
                &[("content-type", "application/json")],
                "{\"prompt\":\"hello\"}",
            ),
            base
        );
    }

    #[test]
    fn the_body_is_covered_through_its_hash_so_two_bodies_of_the_same_length_differ() {
        // A signature that covered only the body length would pass a "change the body" test. Same length, one
        // character different, so only a real content hash can distinguish them.
        let a = authorization_for("POST", "https://example.amazonaws.com/x", &[], "aaaa");
        let b = authorization_for("POST", "https://example.amazonaws.com/x", &[], "aaab");

        assert_eq!(
            "aaaa".len(),
            "aaab".len(),
            "the lengths are equal by construction"
        );
        assert_ne!(
            signature_of(&a),
            signature_of(&b),
            "the payload hash must cover the content, not just its length"
        );
    }

    // ---------------------------------------------------------------------------------------
    // the canonical uri is encoded, which only shows on a path that needs it
    // ---------------------------------------------------------------------------------------

    #[test]
    fn a_path_that_needs_encoding_is_encoded_into_the_canonical_request() {
        // **This test was added because a mutation proved the suite could not see the encoding at all.** Every
        // path used by the other tests contains only unreserved characters, so replacing `aws_uri_encode(uri,
        // false)` with `uri.to_string()` signed identically and nothing failed.
        //
        // The paths below each contain a character the canonical URI must escape. AWS signs the **encoded**
        // path, so an implementation that signed the raw one would produce a signature AWS rejects -- and it
        // would only be noticed on requests whose path happens to contain such a character.
        let base = authorization_for(
            "GET",
            "https://example.amazonaws.com/model/a/invoke",
            &[],
            "",
        );
        let base_sig = signature_of(&base).to_string();
        println!("plain path: {base_sig}");

        for (label, path) in [
            ("a space", "/model/a b/invoke"),
            ("a colon", "/model/a:b/invoke"),
            ("a plus", "/model/a+b/invoke"),
            ("a percent escape of its own", "/model/a%2Fb/invoke"),
            ("a non-ascii segment", "/model/模型/invoke"),
        ] {
            // `?` and `#` are excluded: they are delimiters in a URL, so a path containing them is not the
            // path the client sent and reqwest parses it as a query or fragment instead.
            let url = format!("https://example.amazonaws.com{path}");
            let auth = authorization_for("GET", &url, &[], "");
            let sig = signature_of(&auth).to_string();
            println!("{label}: {path} -> {sig}");
            assert_ne!(
                sig, base_sig,
                "a path containing {label} must produce a different signature, because the canonical uri is \
                 the encoded path. If this matches the plain path, the uri is being signed raw"
            );
        }
    }

    #[test]
    fn two_paths_that_encode_differently_sign_differently() {
        // The converse of the test above, and the one that pins the encoding rather than just its presence:
        // `/a b` encodes to `/a%20b` while `/a%20b` encodes to `/a%2520b`, so these two paths are **not** the
        // same request and must not share a signature. An implementation that passed an already-encoded path
        // through, or that decoded before encoding, would collapse them.
        let spaced = authorization_for("GET", "https://example.amazonaws.com/a%20b", &[], "");
        let literal = authorization_for("GET", "https://example.amazonaws.com/a%2520b", &[], "");

        println!("a%20b   -> {}", signature_of(&spaced));
        println!("a%2520b -> {}", signature_of(&literal));

        assert_ne!(
            signature_of(&spaced),
            signature_of(&literal),
            "a percent sign in the path is escaped as %25, so these are different paths and must sign \
             differently"
        );
    }

    // ---------------------------------------------------------------------------------------
    // the key derivation, anchored to the published test vector
    // ---------------------------------------------------------------------------------------

    #[test]
    fn the_signing_key_matches_the_published_aws_test_vector() {
        // A signature that responded to every input could still be **wrong**, and none of the tests above can
        // tell the difference -- they establish sensitivity, not correctness. This one pins correctness
        // against AWS's own published example.
        //
        // The vector is the `get-vanilla` case from the AWS SigV4 test suite, as published in the AWS
        // documentation's "Examples of the complete Signature Version 4 signing process": credentials
        // `AKIDEXAMPLE` / `wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY`, region `us-east-1`, service `iam`, date
        // 2015-08-30.
        //
        // **It calls the crate's own `signing_key`, not a copy of it.** The first version of this test
        // re-implemented the HMAC chain inline, so it compared its own arithmetic with the vector and could not
        // see a change to the real derivation -- confirmed by mutating the `AWS4` prefix in `lib.rs`, which
        // that version did not catch. `signing_key` was extracted from the signing path so this test
        // exercises the code that actually signs.
        let expected = "c4afb1cc5771d871763a393e44b703571b55cc28424d1a5e86da6ed3c154a4b9";
        let secret = "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY";

        let derived = crate::signing_key(secret, "20150830", "us-east-1", "iam")
            .expect("the derivation accepts any key length");
        let hex_key = hex::encode(&derived);
        println!("derived signing key: {hex_key}");

        assert_eq!(
            hex_key, expected,
            "the published AWS example derives this key; a mismatch means the derivation -- most likely the \
             `AWS4` prefix on the secret, or the order of the chain -- is wrong"
        );

        // And every input reaches the result, so the match above is not the coincidence of a function that
        // ignores its arguments.
        for (label, derived) in [
            (
                "the secret",
                crate::signing_key(
                    "aDifferentSecret000000000000000000000000",
                    "20150830",
                    "us-east-1",
                    "iam",
                ),
            ),
            (
                "the date",
                crate::signing_key(secret, "20150831", "us-east-1", "iam"),
            ),
            (
                "the region",
                crate::signing_key(secret, "20150830", "eu-west-1", "iam"),
            ),
            (
                "the service",
                crate::signing_key(secret, "20150830", "us-east-1", "s3"),
            ),
        ] {
            let hex_derived = hex::encode(derived.expect("the derivation accepts any key length"));
            assert_ne!(
                hex_derived, expected,
                "changing {label} must change the signing key"
            );
        }
    }

    // ---------------------------------------------------------------------------------------
    // query strings
    // ---------------------------------------------------------------------------------------

    #[test]
    fn query_parameters_are_sorted_so_the_order_in_the_url_does_not_change_the_signature() {
        // SigV4 requires the canonical query string to be sorted, so `?b=2&a=1` and `?a=1&b=2` describe the
        // same request and must sign identically. If they did not, AWS would reject one of two equivalent
        // URLs.
        let one = authorization_for("GET", "https://example.amazonaws.com/x?b=2&a=1", &[], "");
        let two = authorization_for("GET", "https://example.amazonaws.com/x?a=1&b=2", &[], "");

        println!("b=2&a=1 -> {}", signature_of(&one));
        println!("a=1&b=2 -> {}", signature_of(&two));

        assert_eq!(
            signature_of(&one),
            signature_of(&two),
            "the same parameters in a different order must produce the same signature"
        );

        // The control for the other direction: a different set of parameters must differ.
        let other = authorization_for("GET", "https://example.amazonaws.com/x?a=1&b=3", &[], "");
        assert_ne!(signature_of(&one), signature_of(&other));
    }

    // ---------------------------------------------------------------------------------------
    // signed headers
    // ---------------------------------------------------------------------------------------

    #[test]
    fn the_signed_header_list_is_lowercase_and_sorted() {
        // The canonical request lists the signed headers semicolon-separated in sorted order, and a sig that
        // were not sorted or not lowercase would be rejected. Headers are supplied here in a deliberately
        // unsorted, mixed-case form.
        let auth = authorization_for(
            "GET",
            "https://example.amazonaws.com/x",
            &[("Content-Type", "application/json")],
            "",
        );

        let signed = auth
            .split("SignedHeaders=")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .expect("SignedHeaders is present");

        println!("signed headers: {signed}");

        assert_eq!(
            signed, "content-type;host;x-amz-date",
            "lowercased and sorted; `content-type` was supplied as `Content-Type`"
        );

        let mut sorted = signed.split(';').collect::<Vec<_>>();
        let original = sorted.clone();
        sorted.sort();
        assert_eq!(sorted, original, "and it is in sorted order");
        assert!(
            signed.chars().all(|c| !c.is_ascii_uppercase()),
            "and contains no uppercase"
        );
    }

    #[test]
    fn a_header_that_is_not_signed_does_not_change_the_signature() {
        // The counterpart of the test above: the implementation signs `host`, `x-amz-date` and `content-type`
        // only, so any other header must leave the signature alone. Recorded because it is a **limitation**
        // rather than a feature -- an unsigned header can be changed in transit without invalidating the
        // request -- and because a signature that moved for every header would be a surprise in the other
        // direction.
        let without = authorization_for("GET", "https://example.amazonaws.com/x", &[], "");
        let with = authorization_for(
            "GET",
            "https://example.amazonaws.com/x",
            &[("x-custom-header", "something")],
            "",
        );

        println!("without: {}", signature_of(&without));
        println!("with:    {}", signature_of(&with));

        assert_eq!(
            signature_of(&without),
            signature_of(&with),
            "an unsigned header does not enter the canonical request, so the signature is unchanged"
        );
        assert!(
            !with.contains("x-custom-header"),
            "and it is not listed as signed: it reads {with}"
        );
    }

    // ---------------------------------------------------------------------------------------
    // uri encoding
    // ---------------------------------------------------------------------------------------

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

    #[test]
    fn utf8_characters_are_encoded_byte_by_byte_in_uppercase_hex() {
        // The plan asks for "UTF-8/百分号/斜杠编码". Each multi-byte character must become one `%XX` per byte:
        // `é` is two bytes, `中` is three and an emoji is four. A naive implementation that pushed the
        // character rather than its bytes, or that encoded the code point as one value, would differ.
        let cases: Vec<(&str, &str, &str)> = vec![
            ("é (2 bytes)", "é", "%C3%A9"),
            ("中 (3 bytes)", "中", "%E4%B8%AD"),
            ("emoji (4 bytes)", "🙂", "%F0%9F%99%82"),
            ("mixed with ascii", "aé/b", "a%C3%A9/b"),
        ];

        for (label, input, expected) in cases {
            let got = aws_uri_encode(input, false);
            println!("{label}: {input:?} -> {got}");
            assert_eq!(got, expected, "{label}");

            // Uppercase hex, checked on the **escapes** rather than on the whole string: an earlier version of
            // this assertion compared the output with its own `to_uppercase()`, which is false for every input
            // containing an unescaped lowercase letter (`a%C3%A9/b` would have to become `A%C3%A9/B`). It was
            // an assertion that could not pass and did not check what it claimed.
            let mut escapes = Vec::new();
            let bytes = got.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'%' {
                    let hex = &got[i + 1..i + 3];
                    escapes.push(hex.to_string());
                    assert!(
                        hex.chars()
                            .all(|c| c.is_ascii_digit() || ('A'..='F').contains(&c)),
                        "{label}: the escape `%{hex}` must use uppercase hex digits"
                    );
                }
                i += 1;
            }
            assert!(
                !escapes.is_empty(),
                "{label}: the case must contain an escape"
            );
        }
    }

    #[test]
    fn characters_aws_calls_unreserved_are_left_alone_and_everything_else_is_escaped() {
        // The exact set is `A-Z a-z 0-9 - . _ ~`. Anything outside it must be escaped, including characters
        // that a generic URL encoder would sometimes leave, and including `%` itself -- which is why a value
        // that already looks encoded gets double-encoded rather than passed through.
        assert_eq!(
            aws_uri_encode("AZaz09-._~", false),
            "AZaz09-._~",
            "the unreserved set is untouched"
        );

        for (input, expected) in [
            ("%", "%25"),
            ("+", "%2B"),
            ("=", "%3D"),
            ("&", "%26"),
            ("?", "%3F"),
            ("#", "%23"),
            ("!", "%21"),
            ("*", "%2A"),
            ("'", "%27"),
            ("(", "%28"),
            (")", "%29"),
        ] {
            let got = aws_uri_encode(input, false);
            assert_eq!(got, expected, "{input:?} must be escaped");
        }

        // A percent sequence is escaped character by character, so it does not survive as a sequence.
        assert_eq!(
            aws_uri_encode("%2F", false),
            "%252F",
            "an existing escape is itself escaped"
        );
    }

    // ---------------------------------------------------------------------------------------
    // credential parsing
    // ---------------------------------------------------------------------------------------

    #[test]
    fn test_aws_config_parsing() {
        let config = AwsConfig::from_colon_string("AK:SK:us-east-1")
            .unwrap_or_else(|e| panic!("failed to parse valid AwsConfig: {e}"));
        assert_eq!(config.access_key, "AK");
        assert_eq!(config.secret_key, "SK");
        assert_eq!(config.region, "us-east-1");

        assert!(AwsConfig::from_colon_string("InvalidString").is_err());
    }

    #[test]
    fn the_credential_parser_accepts_extra_segments_and_ignores_them() {
        // The plan asks to "先明确多段/空段凭证的接受规则" -- state the rule for multi-segment and empty-segment
        // credentials rather than leaving it to be discovered. The rule the code implements is: split on `:`,
        // require at least three parts, take the first three and ignore the rest.
        //
        // That matters because an AWS **secret** access key is base64 and can contain `+` and `/` but not `:`,
        // so a fourth segment is not part of any real credential -- it is a malformed input that is accepted.
        let config = AwsConfig::from_colon_string("AK:SK:us-east-1:EXTRA:ANOTHER").unwrap();
        assert_eq!(config.access_key, "AK");
        assert_eq!(config.secret_key, "SK");
        assert_eq!(config.region, "us-east-1");
        println!("a four-segment credential is accepted and the extra segments are ignored");

        // Recorded as the rule, not endorsed: silently ignoring input is how a mis-set environment variable
        // becomes a signing failure against AWS rather than a configuration error at startup.
    }

    #[test]
    fn the_credential_parser_accepts_empty_segments() {
        // The other half of the rule. `":SK:us-east-1"` has three parts and is therefore valid, with an empty
        // access key -- which will sign requests that AWS rejects with a confusing error rather than a
        // configuration one.
        let config = AwsConfig::from_colon_string(":SK:us-east-1").unwrap();
        assert_eq!(config.access_key, "", "an empty access key is accepted");
        assert_eq!(config.secret_key, "SK");

        let empty_secret = AwsConfig::from_colon_string("AK::us-east-1").unwrap();
        assert_eq!(empty_secret.secret_key, "", "and so is an empty secret key");

        let all_empty = AwsConfig::from_colon_string("::").unwrap();
        assert_eq!(all_empty.access_key, "");
        assert_eq!(all_empty.secret_key, "");
        assert_eq!(all_empty.region, "");

        println!("empty segments are accepted; only the segment count is checked");
    }

    #[test]
    fn the_credential_parser_rejects_fewer_than_three_segments() {
        // The one rule it does enforce, at each boundary. `AwsConfig::from_colon_string` is the only parse of
        // this format in the crate, so this is where a malformed environment variable is caught.
        for input in ["", "AK", "AK:SK", "AK:SK:".to_string().as_str()] {
            let result = AwsConfig::from_colon_string(input);
            if input == "AK:SK:" {
                assert!(
                    result.is_ok(),
                    "`AK:SK:` has three parts, the third empty, so it is accepted -- empty segments are not \
                     rejected"
                );
            } else {
                assert!(
                    result.is_err(),
                    "{input:?} has fewer than three segments and must be rejected"
                );
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // the boundaries that matter downstream
    // ---------------------------------------------------------------------------------------

    #[test]
    fn a_region_with_an_unusual_spelling_is_signed_verbatim() {
        // The region goes into the credential scope unmodified, so the signature must follow it. Worth pinning
        // because a normalising implementation would hide a mis-set region instead of signing for the wrong
        // one -- which AWS rejects, and which is easier to diagnose than a silently corrected region.
        let mut cfg = config();
        cfg.region = "eu-west-1".to_string();
        let eu = {
            let mut request = reqwest::Client::new()
                .get("https://example.amazonaws.com/")
                .build()
                .unwrap();
            sign_request_at(&mut request, &cfg, b"", fixed_clock()).unwrap();
            request
                .headers()
                .get("authorization")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        };

        assert!(
            eu.contains("/eu-west-1/bedrock/aws4_request"),
            "the region appears in the scope as given: {eu}"
        );
        assert_ne!(
            signature_of(&eu),
            signature_of(&authorization_for("GET", "https://example.amazonaws.com/", &[], "")),
            "and a different region produces a different signature, because the scope is part of the string to \
             sign"
        );
    }

    #[test]
    fn the_access_key_appears_in_the_header_but_not_in_the_signature() {
        // The access key identifies the caller; the **secret** is what signs. Two configs differing only in
        // the access key must therefore produce the same signature -- which is worth stating, because the
        // opposite mistake (signing with the access key) would look correct in the header and fail at AWS.
        let mut other = config();
        other.access_key = "ADIFFERENTKEY".to_string();

        let mut request = reqwest::Client::new()
            .get("https://example.amazonaws.com/")
            .build()
            .unwrap();
        sign_request_at(&mut request, &other, b"", fixed_clock()).unwrap();
        let auth = request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        println!("different access key: {auth}");
        assert!(
            auth.contains("Credential=ADIFFERENTKEY/"),
            "the access key is named in the credential"
        );
        assert_eq!(
            signature_of(&auth),
            signature_of(&authorization_for(
                "GET",
                "https://example.amazonaws.com/",
                &[],
                ""
            )),
            "but the signature depends on the secret, not the access key"
        );
    }

    #[test]
    fn a_different_secret_produces_a_different_signature() {
        // The control for the test above: the secret **does** reach the signature, so "the access key does
        // not" is a statement about the access key rather than about signing being insensitive to credentials.
        let mut other = config();
        other.secret_key = "aCompletelyDifferentSecretKeyValue000000000000".to_string();

        let mut request = reqwest::Client::new()
            .get("https://example.amazonaws.com/")
            .build()
            .unwrap();
        sign_request_at(&mut request, &other, b"", fixed_clock()).unwrap();
        let auth = request
            .headers()
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        assert_ne!(
            signature_of(&auth),
            signature_of(&authorization_for(
                "GET",
                "https://example.amazonaws.com/",
                &[],
                ""
            )),
            "the secret is the signing key, so it must change the signature"
        );
    }
}
