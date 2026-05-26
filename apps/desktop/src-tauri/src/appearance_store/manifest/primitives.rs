use std::collections::HashMap;

pub(super) fn validate_localized_text_map(
    label: &str,
    values: &HashMap<String, String>,
    max_len: usize,
) -> Result<(), String> {
    for (locale, text) in values {
        validate_locale_key(label, locale)?;
        validate_non_empty(label, text)?;
        if text.len() > max_len {
            return Err(format!("{label} value is too long"));
        }
    }
    Ok(())
}

pub(super) fn validate_locale_key(label: &str, locale: &str) -> Result<(), String> {
    if locale.is_empty()
        || locale.len() > 16
        || !locale
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_')
    {
        return Err(format!("{label} contains an invalid locale key: {locale}"));
    }
    Ok(())
}

pub(super) fn validate_non_empty(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} cannot be empty"))
    } else {
        Ok(())
    }
}

pub(super) fn validate_dotted_identifier(
    label: &str,
    value: &str,
    require_dot: bool,
) -> Result<(), String> {
    if require_dot && !value.contains('.') {
        return Err(format!("{label} must use a dotted identifier"));
    }
    if value.split('.').all(is_identifier_segment) {
        Ok(())
    } else {
        Err(format!("{label} is invalid: {value}"))
    }
}

fn is_identifier_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-')
}

pub(super) fn validate_simple_semver(label: &str, value: &str) -> Result<(), String> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|char| char.is_ascii_digit()))
    {
        Ok(())
    } else {
        Err(format!("{label} must use major.minor.patch"))
    }
}

fn parse_simple_semver(value: &str) -> Result<[u64; 3], String> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("invalid semver: {value}"));
    }
    let major = parts[0]
        .parse::<u64>()
        .map_err(|_| format!("invalid semver: {value}"))?;
    let minor = parts[1]
        .parse::<u64>()
        .map_err(|_| format!("invalid semver: {value}"))?;
    let patch = parts[2]
        .parse::<u64>()
        .map_err(|_| format!("invalid semver: {value}"))?;
    Ok([major, minor, patch])
}

pub(super) fn compare_simple_semver(left: &str, right: &str) -> Result<std::cmp::Ordering, String> {
    Ok(parse_simple_semver(left)?.cmp(&parse_simple_semver(right)?))
}

pub(super) fn validate_http_url(label: &str, value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    validate_non_empty(label, trimmed)?;
    if trimmed.len() > 2048 || trimmed.chars().any(char::is_whitespace) {
        return Err(format!("{label} is invalid"));
    }
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return Err(format!("{label} must use http or https"));
    };
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
        return Err(format!("{label} must use http or https"));
    }
    if rest.trim_matches('/').is_empty() {
        return Err(format!("{label} must include a host"));
    }
    Ok(())
}

pub(super) fn validate_color_token(token: &str, value: &str) -> Result<(), String> {
    let value = value.trim();
    if is_hex_color(value) || is_rgba_color(value) {
        Ok(())
    } else {
        Err(format!("{token} color is invalid: {value}"))
    }
}

fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 6) && hex.chars().all(|char| char.is_ascii_hexdigit())
}

fn is_rgba_color(value: &str) -> bool {
    let Some(inner) = value
        .strip_prefix("rgba(")
        .and_then(|value| value.strip_suffix(')'))
    else {
        return false;
    };
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() != 4 {
        return false;
    }

    let rgb_ok = parts[..3]
        .iter()
        .all(|part| part.parse::<u16>().is_ok_and(|value| value <= 255));
    let alpha_ok = parts[3]
        .parse::<f64>()
        .is_ok_and(|value| (0.0..=1.0).contains(&value));
    rgb_ok && alpha_ok
}
