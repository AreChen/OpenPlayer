use std::{collections::BTreeMap, io::Read};

#[derive(Debug, Clone)]
pub(in crate::mpv_embed) struct RtspUrl {
    pub(in crate::mpv_embed) host: String,
    pub(in crate::mpv_embed) port: u16,
    pub(in crate::mpv_embed) request_url: String,
}

#[derive(Debug)]
pub(in crate::mpv_embed) struct RtspResponse {
    pub(in crate::mpv_embed) status_line: String,
    pub(in crate::mpv_embed) headers: BTreeMap<String, String>,
    pub(in crate::mpv_embed) body: String,
}

pub(in crate::mpv_embed) fn parse_rtsp_url(value: &str) -> Option<RtspUrl> {
    let rest = value.strip_prefix("rtsp://")?;
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let (host, port) = if let Some(host) = authority.strip_prefix('[') {
        let (host, tail) = host.split_once(']')?;
        let port = tail
            .strip_prefix(':')
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(554);
        (host.to_string(), port)
    } else if let Some((host, port)) = authority.rsplit_once(':') {
        (host.to_string(), port.parse::<u16>().ok().unwrap_or(554))
    } else {
        (authority.to_string(), 554)
    };
    if host.is_empty() {
        return None;
    }
    Some(RtspUrl {
        host,
        port,
        request_url: format!("rtsp://{authority}/{}", path.trim_start_matches('/')),
    })
}

#[cfg(test)]
pub(in crate::mpv_embed) fn rtsp_rtp_info_timestamp(value: &str) -> Option<u32> {
    value
        .split([';', ','])
        .find_map(|part| part.trim().strip_prefix("rtptime=")?.parse::<u32>().ok())
}

pub(in crate::mpv_embed) fn rtsp_track_url(base_url: &str, sdp: &str) -> String {
    let control = sdp
        .lines()
        .find_map(|line| line.trim().strip_prefix("a=control:"))
        .unwrap_or("trackID=0")
        .trim();
    if control.starts_with("rtsp://") {
        control.to_string()
    } else {
        format!(
            "{}/{}",
            base_url.trim_end_matches('/'),
            control.trim_start_matches('/')
        )
    }
}

pub(in crate::mpv_embed) fn parse_rtsp_response(head: &str, body: Vec<u8>) -> RtspResponse {
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default().to_string();
    let headers = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect();
    RtspResponse {
        status_line,
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    }
}

pub(in crate::mpv_embed) fn read_rtsp_response(
    stream: &mut impl Read,
) -> std::io::Result<RtspResponse> {
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    while !header.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte)?;
        header.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&header[..header.len().saturating_sub(4)]).into_owned();
    let content_length = head
        .split("\r\n")
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        stream.read_exact(&mut body)?;
    }
    Ok(parse_rtsp_response(&head, body))
}
