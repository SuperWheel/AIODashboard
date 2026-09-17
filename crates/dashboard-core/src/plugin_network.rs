//! GET 代理：规范 URL、解析并固定 DNS、禁止重定向、流式限额。
use crate::{CoreError, CoreResult};
use std::{
    io::{self, Read},
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};
use url::Url;
pub const RESPONSE_LIMIT: u64 = 2_000_000;
const TIMEOUT: Duration = Duration::from_secs(15);
static DNS_ACTIVE: AtomicUsize = AtomicUsize::new(0);

pub fn parse_url(raw: &str) -> CoreResult<Url> {
    if raw.len() > 8192 || raw.chars().any(|c| c.is_control()) {
        return Err(CoreError::Validation("URL 过长或含控制字符".into()));
    }
    let url = Url::parse(raw).map_err(|_| CoreError::Validation("非法 URL".into()))?;
    if !["http", "https"].contains(&url.scheme())
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(CoreError::Validation(
            "仅支持无凭据的 HTTP/HTTPS URL".into(),
        ));
    }
    Ok(url)
}

pub fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            let [a, b, _, _] = v.octets();
            !(v.is_private()
                || v.is_loopback()
                || v.is_link_local()
                || v.is_unspecified()
                || v.is_broadcast()
                || v.is_documentation()
                || a == 0
                || a >= 224
                || (a == 100 && (64..=127).contains(&b))
                || (a == 192 && b == 0)
                || (a == 198 && (b == 18 || b == 19)))
        }
        IpAddr::V6(v) => {
            if let Some(v4) = v.to_ipv4_mapped() {
                return public_ip(IpAddr::V4(v4));
            }
            let s = v.segments();
            // 仅全球单播，排除特殊/文档/6to4、Teredo 等转换范围。
            (s[0] & 0xe000) == 0x2000
                && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
                && s[0] != 0x2002
        }
    }
}

pub fn allowed_url(allowlist: &[String], raw: &str) -> CoreResult<Url> {
    let url = parse_url(raw)?;
    let host = url.host_str().unwrap_or_default();
    let local = host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local");
    let ip = host.trim_matches(['[', ']']).parse::<IpAddr>().ok();
    if local || ip.is_some_and(|ip| !public_ip(ip)) || !allowlist.iter().any(|h| h == host) {
        return Err(CoreError::PermissionDenied(format!(
            "network 未授权或私有目标: {host}"
        )));
    }
    Ok(url)
}

fn resolve(url: &Url) -> CoreResult<Vec<SocketAddr>> {
    if DNS_ACTIVE
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
            (n < 4).then_some(n + 1)
        })
        .is_err()
    {
        return Err(CoreError::Validation("DNS 请求并发达到上限".into()));
    }
    let host = url
        .host_str()
        .unwrap_or_default()
        .trim_matches(['[', ']'])
        .to_string();
    let port = url.port_or_known_default().unwrap_or(443);
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = (host.as_str(), port)
            .to_socket_addrs()
            .map(|v| v.collect::<Vec<_>>());
        DNS_ACTIVE.fetch_sub(1, Ordering::SeqCst);
        let _ = tx.send(result);
    });
    let addresses = rx
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| CoreError::Validation("DNS 超时".into()))?
        .map_err(|_| CoreError::Validation("DNS 解析失败".into()))?;
    validate_addresses(&addresses)?;
    Ok(addresses)
}
pub fn validate_addresses(addresses: &[SocketAddr]) -> CoreResult<()> {
    if addresses.is_empty() || addresses.iter().any(|a| !public_ip(a.ip())) {
        return Err(CoreError::PermissionDenied(
            "DNS 包含本机、私有或特殊地址".into(),
        ));
    }
    Ok(())
}
fn bounded_text(reader: impl Read) -> CoreResult<String> {
    let mut bytes = Vec::new();
    reader
        .take(RESPONSE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| CoreError::Validation(format!("HTTP 读取失败: {e}")))?;
    if bytes.len() as u64 > RESPONSE_LIMIT {
        return Err(CoreError::Validation("HTTP 响应超过 2MB".into()));
    }
    String::from_utf8(bytes).map_err(|_| CoreError::Validation("响应不是 UTF-8 文本".into()))
}
pub fn fetch(allowlist: &[String], raw: &str) -> CoreResult<serde_json::Value> {
    let url = allowed_url(allowlist, raw)?;
    let addresses = resolve(&url)?;
    fetch_pinned(&url, addresses, TIMEOUT)
}
fn fetch_pinned(
    url: &Url,
    addresses: Vec<SocketAddr>,
    timeout: Duration,
) -> CoreResult<serde_json::Value> {
    let host = url.host_str().unwrap_or_default().to_string();
    let expected = format!("{}:{}", host, url.port_or_known_default().unwrap_or(443));
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(timeout)
        .timeout_connect(Duration::from_secs(5))
        .try_proxy_from_env(false)
        .resolver(move |name: &str| -> io::Result<Vec<SocketAddr>> {
            if name != expected {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "DNS 目标发生变化",
                ));
            }
            Ok(addresses.clone())
        })
        .build();
    let response = match agent.get(url.as_str()).call() {
        Ok(r) | Err(ureq::Error::Status(_, r)) => r,
        Err(_) => return Err(CoreError::Validation("HTTP 请求失败或超时".into())),
    };
    let status = response.status();
    if (300..400).contains(&status) {
        return Err(CoreError::PermissionDenied("HTTP 重定向被禁止".into()));
    }
    if response
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .is_some_and(|n| n > RESPONSE_LIMIT)
    {
        return Err(CoreError::Validation("HTTP 响应超过 2MB".into()));
    }
    let text = bounded_text(response.into_reader())?;
    let json = serde_json::from_str::<serde_json::Value>(&text).ok();
    Ok(serde_json::json!({"status":status,"text":text,"json":json}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ssrf_forms_and_dns_mixed_answers() {
        for raw in [
            "http://127.1",
            "http://2130706433",
            "http://[::ffff:127.0.0.1]",
            "http://localhost",
            "http://169.254.169.254",
        ] {
            let host = parse_url(raw).unwrap().host_str().unwrap().to_string();
            assert!(allowed_url(&[host], raw).is_err());
        }
        for raw in [
            "file:///x",
            "http://good.example:80@evil.example",
            "https://u:p@good.example",
        ] {
            assert!(allowed_url(&["good.example".into()], raw).is_err());
        }
        assert!(validate_addresses(&[
            "8.8.8.8:443".parse().unwrap(),
            "10.0.0.1:443".parse().unwrap()
        ])
        .is_err());
        assert!(allowed_url(&["api.example.com".into()], "https://API.example.com:443/x").is_ok());
    }
    #[test]
    fn actual_http_success_redirect_and_timeout() {
        use std::io::Write;
        for response in [
            Some("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}"),
            Some("HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1/\r\nContent-Length: 0\r\n\r\n"),
            None,
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut buf = [0; 1024];
                let _ = socket.read(&mut buf);
                if let Some(r) = response {
                    socket.write_all(r.as_bytes()).unwrap();
                } else {
                    std::thread::sleep(Duration::from_millis(200));
                }
            });
            let url = Url::parse(&format!("http://api.example.com:{}/", address.port())).unwrap();
            let result = fetch_pinned(&url, vec![address], Duration::from_millis(50));
            if response.is_some_and(|r| r.contains("200 OK")) {
                assert_eq!(result.unwrap()["status"], 200);
            } else {
                assert!(result.is_err());
            }
            server.join().unwrap();
        }
    }
    #[test]
    fn streaming_limit() {
        assert!(bounded_text(io::repeat(b'x').take(RESPONSE_LIMIT + 1)).is_err());
    }
}
