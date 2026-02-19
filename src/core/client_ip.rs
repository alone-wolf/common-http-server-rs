//! Client IP extraction helpers.
//!
//! The helpers support common proxy headers (`Forwarded`, `X-Forwarded-For`,
//! `X-Real-IP`, `CF-Connecting-IP`) and can enforce trusted-proxy boundaries.

use axum::http::HeaderMap;
use ipnet::IpNet;
use std::{net::IpAddr, str::FromStr};

pub fn extract_client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    extract_from_forwarded(headers)
        .or_else(|| extract_from_x_forwarded_for(headers))
        .or_else(|| extract_single_header(headers, "x-real-ip"))
        .or_else(|| extract_single_header(headers, "cf-connecting-ip"))
}

pub fn extract_client_ip_with_trusted_proxies(
    headers: &HeaderMap,
    peer_ip: Option<IpAddr>,
    trusted_proxies: &[IpNet],
) -> Option<IpAddr> {
    if let Some(peer_ip) = peer_ip
        && !is_trusted_proxy(peer_ip, trusted_proxies)
    {
        return Some(peer_ip);
    }

    let forwarded_chain = extract_forwarded_chain(headers);
    if !forwarded_chain.is_empty() {
        return select_client_ip_from_chain(&forwarded_chain, trusted_proxies);
    }

    if let Some(ip) = extract_single_header(headers, "x-real-ip")
        .or_else(|| extract_single_header(headers, "cf-connecting-ip"))
    {
        return Some(ip);
    }

    peer_ip
}

fn extract_from_forwarded(headers: &HeaderMap) -> Option<IpAddr> {
    let forwarded = headers.get("forwarded")?.to_str().ok()?;

    for part in forwarded.split(',') {
        for token in part.split(';') {
            let token = token.trim();
            if let Some(value) = token.strip_prefix("for=")
                && let Some(ip) = parse_ip_value(value)
            {
                return Some(ip);
            }
        }
    }

    None
}

fn extract_from_x_forwarded_for(headers: &HeaderMap) -> Option<IpAddr> {
    let forwarded_for = headers.get("x-forwarded-for")?.to_str().ok()?;

    for candidate in forwarded_for.split(',') {
        if let Some(ip) = parse_ip_value(candidate) {
            return Some(ip);
        }
    }

    None
}

fn extract_forwarded_chain(headers: &HeaderMap) -> Vec<IpAddr> {
    let mut chain = Vec::new();

    if let Some(forwarded) = headers.get("forwarded").and_then(|h| h.to_str().ok()) {
        for part in forwarded.split(',') {
            for token in part.split(';') {
                let token = token.trim();
                if let Some(value) = token.strip_prefix("for=")
                    && let Some(ip) = parse_ip_value(value)
                {
                    chain.push(ip);
                }
            }
        }
    }

    if chain.is_empty()
        && let Some(xff) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok())
    {
        for candidate in xff.split(',') {
            if let Some(ip) = parse_ip_value(candidate) {
                chain.push(ip);
            }
        }
    }

    chain
}

fn select_client_ip_from_chain(chain: &[IpAddr], trusted_proxies: &[IpNet]) -> Option<IpAddr> {
    for ip in chain.iter().rev() {
        if !is_trusted_proxy(*ip, trusted_proxies) {
            return Some(*ip);
        }
    }

    chain.first().copied()
}

fn is_trusted_proxy(ip: IpAddr, trusted_proxies: &[IpNet]) -> bool {
    trusted_proxies.iter().any(|net| net.contains(&ip))
}

fn extract_single_header(headers: &HeaderMap, name: &str) -> Option<IpAddr> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_ip_value)
}

fn parse_ip_value(raw: &str) -> Option<IpAddr> {
    let value = raw.trim().trim_matches('"');

    // Some proxy implementations forward tokens like "for=1.2.3.4".
    if let Some((_, rest)) = value.split_once('=') {
        return parse_ip_value(rest);
    }

    // Handle bracketed IPv6 addresses with optional port, e.g. "[2001:db8::1]:443".
    if let Some(stripped) = value.strip_prefix('[')
        && let Some((host, suffix)) = stripped.split_once(']')
        && (suffix.is_empty() || suffix.starts_with(':'))
        && let Ok(ip) = IpAddr::from_str(host)
    {
        return Some(ip);
    }

    let value = value.strip_prefix('[').unwrap_or(value);
    let value = value.strip_suffix(']').unwrap_or(value);

    IpAddr::from_str(value).ok().or_else(|| {
        // Try "ip:port" only when the trailing part is numeric, so plain IPv6
        // addresses like "2001:db8::1" are not misinterpreted.
        value.rsplit_once(':').and_then(|(host, port)| {
            if port.chars().all(|c| c.is_ascii_digit()) {
                IpAddr::from_str(host).ok()
            } else {
                None
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn extract_from_x_forwarded_for_prefers_first() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.1, 198.51.100.22"),
        );

        assert_eq!(
            extract_client_ip(&headers),
            Some("203.0.113.1".parse().unwrap())
        );
    }

    #[test]
    fn extract_from_forwarded_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "forwarded",
            HeaderValue::from_static("for=203.0.113.2;proto=https"),
        );

        assert_eq!(
            extract_client_ip(&headers),
            Some("203.0.113.2".parse().unwrap())
        );
    }

    #[test]
    fn extract_from_forwarded_header_ipv6_with_port() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "forwarded",
            HeaderValue::from_static("for=\"[2001:db8::10]:443\";proto=https"),
        );

        assert_eq!(
            extract_client_ip(&headers),
            Some("2001:db8::10".parse().unwrap())
        );
    }

    #[test]
    fn extract_from_x_forwarded_for_ipv6_with_port() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("[2001:db8::20]:8443, 198.51.100.22"),
        );

        assert_eq!(
            extract_client_ip(&headers),
            Some("2001:db8::20".parse().unwrap())
        );
    }

    #[test]
    fn untrusted_peer_ignores_forwarded_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.10, 198.51.100.1"),
        );
        let peer = Some("198.18.0.1".parse().unwrap());
        let trusted: Vec<IpNet> = vec!["10.0.0.0/8".parse().unwrap()];

        assert_eq!(
            extract_client_ip_with_trusted_proxies(&headers, peer, &trusted),
            Some("198.18.0.1".parse().unwrap())
        );
    }

    #[test]
    fn trusted_peer_uses_proxy_chain_boundary() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.10, 10.1.1.1"),
        );
        let peer = Some("10.1.1.2".parse().unwrap());
        let trusted: Vec<IpNet> = vec!["10.0.0.0/8".parse().unwrap()];

        assert_eq!(
            extract_client_ip_with_trusted_proxies(&headers, peer, &trusted),
            Some("203.0.113.10".parse().unwrap())
        );
    }
}
