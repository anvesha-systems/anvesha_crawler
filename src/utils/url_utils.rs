use url::Url;

pub fn normalize_url(url: &str) -> Result<String, url::ParseError> {
    let parsed = Url::parse(url)?;
    Ok(parsed.to_string())
}

pub fn extract_domain(url: &str) -> Option<String> {
    Url::parse(url).ok()?.host_str().map(|h| h.to_string())
}

pub fn is_same_domain(url1: &str, url2: &str) -> bool {
    match (extract_domain(url1), extract_domain(url2)) {
        (Some(d1), Some(d2)) => d1 == d2,
        _ => false,
    }
}
