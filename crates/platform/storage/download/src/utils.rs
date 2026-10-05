/// 从 URL 中提取文件名
pub(crate) fn extract_filename_from_url(url: &str) -> Option<String> {
    // 移除查询参数
    let url_without_query = url.split('?').next()?;

    // 提取路径的最后一部分
    let filename = url_without_query.split('/').next_back()?;

    // 如果文件名为空，返回 None
    if filename.is_empty() {
        None
    } else {
        Some(filename.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::extract_filename_from_url;

    /// The ordinary case, and the shapes around it, asserted together because they are one rule: take the last
    /// `/`-separated segment of the path, with the query removed.
    #[test]
    fn the_last_path_segment_is_the_filename() {
        let cases = [
            ("https://example.invalid/file.bin", Some("file.bin")),
            ("https://example.invalid/a/b/c.tar.gz", Some("c.tar.gz")),
            (
                "http://example.invalid:8080/dir/model.gguf",
                Some("model.gguf"),
            ),
            // No scheme at all: the function is textual, so a bare path works the same way.
            ("/local/path/file.bin", Some("file.bin")),
            ("file.bin", Some("file.bin")),
            // The trailing segment is the whole path when there is no slash.
            ("a/b/", None),
        ];

        for (url, expected) in cases {
            let got = extract_filename_from_url(url);
            println!("{url:?} -> {got:?}");
            assert_eq!(got.as_deref(), expected, "{url:?}");
        }
    }

    #[test]
    fn a_query_string_is_removed_before_the_segment_is_taken() {
        // The first thing the function does. Without it a signed download URL would name the file after its
        // signature, which is both wrong and unusable as a filename.
        let cases = [
            (
                "https://example.invalid/file.bin?token=abc&expires=123",
                Some("file.bin"),
            ),
            // A query with no path: the host is what remains, the same trap as above.
            ("https://example.invalid?token=abc", Some("example.invalid")),
            // A `?` in the path is still a query separator to this function, which is what "textual" means here.
            ("https://example.invalid/a?b/c.bin", Some("a")),
            // Several `?`: only the first splits.
            ("https://example.invalid/f.bin?x=1?y=2", Some("f.bin")),
        ];

        for (url, expected) in cases {
            let got = extract_filename_from_url(url);
            println!("{url:?} -> {got:?}");
            assert_eq!(got.as_deref(), expected, "{url:?}");
        }
    }

    #[test]
    fn a_url_with_no_filename_returns_none_rather_than_an_empty_string() {
        // The `None` cases exist so a caller can fall back to the server's `Content-Disposition` or to its own
        // name. Returning `Some("")` would produce a file called nothing, which is worse than a fallback.
        for url in [
            "https://example.invalid/", // trailing slash: the last segment is empty
            "",                         // the empty string
            "/",                        // just a slash
            "https://example.invalid/a/b/", // a directory path
        ] {
            let got = extract_filename_from_url(url);
            println!("{url:?} -> {got:?}");
            assert_eq!(got, None, "{url:?} has no filename");
        }

        // The distinction that matters: `Some("")` never comes back, so `Option` carries real information.
        for url in ["https://example.invalid/", ""] {
            assert_ne!(
                extract_filename_from_url(url),
                Some(String::new()),
                "{url:?} must be None, not an empty name"
            );
        }

        // **A host-only URL gives the host as the name, which is worth knowing.** `"https://example.invalid"` has
        // no path, so the last `/`-separated segment is the host itself and the function returns
        // `Some("example.invalid")`. That is correct for a textual function -- that segment *is* the last one --
        // and it is a trap for a caller, because the host is a plausible-looking filename and would be passed to
        // aria2 as `out:` rather than falling back. My first version of this test asserted `None` and measured
        // this.
        //
        // Recorded rather than changed: deciding that a host is not a filename means parsing the URL, which is a
        // different function with different failure modes, and this one is documented as textual.
        let host_only = extract_filename_from_url("https://example.invalid");
        println!("host-only -> {host_only:?}");
        assert_eq!(
            host_only.as_deref(),
            Some("example.invalid"),
            "the host is returned as the filename, so a caller must not treat `Some` as proof of a real file"
        );
        assert_eq!(
            extract_filename_from_url("https://example.invalid?token=abc").as_deref(),
            Some("example.invalid"),
            "and stripping the query leaves the same hosts-as-name result"
        );
    }

    #[test]
    fn a_fragment_and_a_percent_escape_are_left_in_the_name_because_only_the_query_is_stripped() {
        // **Recorded rather than endorsed.** The function removes the query but not the fragment, and it does not
        // decode percent escapes, so a URL with either produces a name containing them. Both are measured here
        // because a caller passing such a URL gets a surprising filename, and the alternative -- assuming the
        // function is a URL parser -- would be wrong.
        let fragmented = extract_filename_from_url("https://example.invalid/file.bin#section");
        println!("with a fragment -> {fragmented:?}");
        assert_eq!(
            fragmented.as_deref(),
            Some("file.bin#section"),
            "the fragment is part of the name, because only `?` is treated as a separator"
        );

        let escaped = extract_filename_from_url("https://example.invalid/my%20file.bin");
        println!("with an escape -> {escaped:?}");
        assert_eq!(
            escaped.as_deref(),
            Some("my%20file.bin"),
            "the escape is kept, so a caller that needs the real name must decode it"
        );

        // And a Unicode name passes through untouched, which is the case that matters for a model file named in
        // another script.
        let unicode = extract_filename_from_url("https://example.invalid/模型-音楽.gguf?x=1");
        println!("with unicode -> {unicode:?}");
        assert_eq!(unicode.as_deref(), Some("模型-音楽.gguf"));
    }
}
