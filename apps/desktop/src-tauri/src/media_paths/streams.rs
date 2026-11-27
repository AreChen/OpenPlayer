pub(crate) fn is_hls_manifest_media_url(source: &str) -> bool {
    let Some((scheme, rest)) = source.trim().split_once("://") else {
        return false;
    };
    (scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
        && rest
            .split(['?', '#'])
            .next()
            .and_then(|url| url.split_once('/'))
            .and_then(|(_, path)| path.rsplit_once('.'))
            .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("m3u8"))
}
