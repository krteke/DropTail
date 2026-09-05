use std::collections::HashMap;
use std::io::Read;
use std::net::IpAddr;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::blocking::{Body, Client};
use serde::Deserialize;
use thiserror::Error;

use crate::domain::device::{Connection, Device};

const SOCKET_PATH: &str = "/var/run/tailscale/tailscaled.sock";
const STATUS_URL: &str = "http://local-tailscaled.sock/localapi/v0/status";
// Values from Tailscale's ipnstate.TaildropTargetStatus.
const TAILDROP_AVAILABLE: i32 = 1;
const TAILDROP_OFFLINE: i32 = 5;

pub struct LocalApiClient {
    client: Client,
}

#[derive(Debug, Error)]
pub enum LocalApiError {
    #[error("无法访问 tailscaled LocalAPI: {0}")]
    Request(#[from] reqwest::Error),
    #[error("tailscaled LocalAPI 返回了 HTTP {status}")]
    HttpStatus { status: u16 },
    #[error("tailscaled 拒绝发送文件（HTTP {status}）：{message}")]
    FilePutRejected { status: u16, message: String },
    #[error("tailscaled LocalAPI 返回了无效的状态数据: {0}")]
    InvalidStatus(#[from] serde_json::Error),
    #[error("Tailscale 当前未运行（BackendState: {state}）")]
    NotRunning { state: String },
    #[error("Taildrop 设备“{name}”缺少稳定节点 ID")]
    MissingStableId { name: String },
    #[error("Taildrop 设备“{name}”没有 Tailscale IP")]
    MissingAddress { name: String },
    #[error("在线的 Taildrop 设备“{name}”没有可识别的连接路径")]
    MissingConnection { name: String },
    #[error("Taildrop 设备缺少可显示的名称")]
    MissingName,
}

impl LocalApiClient {
    pub fn new() -> Result<Self, LocalApiError> {
        let client = Client::builder()
            .unix_socket(SOCKET_PATH)
            .timeout(None)
            .connect_timeout(Duration::from_secs(2))
            .user_agent(concat!("droptail/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { client })
    }

    pub fn devices(&self) -> Result<Vec<Device>, LocalApiError> {
        let response = self
            .client
            .get(STATUS_URL)
            .timeout(Duration::from_secs(5))
            .send()?;
        let status = response.status();
        if !status.is_success() {
            return Err(LocalApiError::HttpStatus {
                status: status.as_u16(),
            });
        }

        parse_devices(&response.bytes()?)
    }

    pub fn push_file<R>(
        &self,
        target_id: &str,
        name: &str,
        size: Option<u64>,
        contents: R,
    ) -> Result<(), LocalApiError>
    where
        R: Read + Send + 'static,
    {
        let body = match size {
            Some(size) => Body::sized(contents, size),
            None => Body::new(contents),
        };
        let response = self
            .client
            .put(file_put_url(target_id, name))
            .body(body)
            .send()?;
        let status = response.status();
        if status == StatusCode::OK {
            response.bytes()?;
            return Ok(());
        }

        let message = response.text()?.trim().to_owned();
        Err(LocalApiError::FilePutRejected {
            status: status.as_u16(),
            message,
        })
    }
}

fn file_put_url(target_id: &str, name: &str) -> reqwest::Url {
    let mut url = reqwest::Url::parse("http://local-tailscaled.sock")
        .expect("the LocalAPI origin must be a valid URL");
    url.path_segments_mut()
        .expect("the LocalAPI HTTP URL must support path segments")
        .extend(["localapi", "v0", "file-put", target_id, name]);
    url
}

fn parse_devices(body: &[u8]) -> Result<Vec<Device>, LocalApiError> {
    let status: StatusResponse = serde_json::from_slice(body)?;
    if status.backend_state != "Running" {
        return Err(LocalApiError::NotRunning {
            state: status.backend_state,
        });
    }

    let dns_suffix = status
        .current_tailnet
        .as_ref()
        .map(|tailnet| tailnet.magic_dns_suffix.as_str())
        .filter(|suffix| !suffix.is_empty())
        .unwrap_or(&status.magic_dns_suffix);
    let mut devices = Vec::new();

    for peer in status.peer.unwrap_or_default().into_values() {
        if !matches!(peer.taildrop_target, TAILDROP_AVAILABLE | TAILDROP_OFFLINE) {
            continue;
        }

        let name = display_name(&peer, dns_suffix).ok_or(LocalApiError::MissingName)?;
        if peer.id.is_empty() {
            return Err(LocalApiError::MissingStableId { name });
        }
        let address = preferred_address(&peer.tailscale_ips)
            .ok_or_else(|| LocalApiError::MissingAddress { name: name.clone() })?;
        let connection = connection(&peer)
            .ok_or_else(|| LocalApiError::MissingConnection { name: name.clone() })?;

        devices.push(Device {
            id: peer.id,
            name,
            platform: peer.os,
            address,
            connection,
        });
    }

    Ok(devices)
}

fn display_name(peer: &PeerStatus, dns_suffix: &str) -> Option<String> {
    let dns_name = peer.dns_name.trim_end_matches('.');
    let dns_suffix = dns_suffix.trim_matches('.');
    if !dns_name.is_empty() {
        if !dns_suffix.is_empty()
            && let Some(name) = dns_name
                .strip_suffix(dns_suffix)
                .and_then(|name| name.strip_suffix('.'))
            && !name.is_empty()
        {
            return Some(name.to_owned());
        }
        return Some(dns_name.to_owned());
    }

    let host_name = peer.host_name.trim();
    (!host_name.is_empty()).then(|| host_name.to_owned())
}

fn preferred_address(addresses: &[IpAddr]) -> Option<IpAddr> {
    addresses
        .iter()
        .copied()
        .find(IpAddr::is_ipv4)
        .or_else(|| addresses.first().copied())
}

fn connection(peer: &PeerStatus) -> Option<Connection> {
    if !peer.online {
        return Some(Connection::Offline);
    }
    if !peer.peer_relay.is_empty() {
        Some(Connection::PeerRelay(peer.peer_relay.clone()))
    } else if !peer.cur_addr.is_empty() {
        Some(Connection::Direct)
    } else if !peer.relay.is_empty() {
        Some(Connection::Derp(peer.relay.clone()))
    } else {
        None
    }
}

#[derive(Debug, Deserialize)]
struct StatusResponse {
    #[serde(rename = "BackendState")]
    backend_state: String,
    #[serde(rename = "MagicDNSSuffix", default)]
    magic_dns_suffix: String,
    #[serde(rename = "CurrentTailnet", default)]
    current_tailnet: Option<TailnetStatus>,
    #[serde(rename = "Peer", default)]
    peer: Option<HashMap<String, PeerStatus>>,
}

#[derive(Debug, Deserialize)]
struct TailnetStatus {
    #[serde(rename = "MagicDNSSuffix", default)]
    magic_dns_suffix: String,
}

#[derive(Debug, Deserialize)]
struct PeerStatus {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(rename = "HostName", default)]
    host_name: String,
    #[serde(rename = "DNSName", default)]
    dns_name: String,
    #[serde(rename = "OS", default)]
    os: String,
    #[serde(rename = "TailscaleIPs", default)]
    tailscale_ips: Vec<IpAddr>,
    #[serde(rename = "CurAddr", default)]
    cur_addr: String,
    #[serde(rename = "Relay", default)]
    relay: String,
    #[serde(rename = "PeerRelay", default)]
    peer_relay: String,
    #[serde(rename = "Online", default)]
    online: bool,
    #[serde(rename = "TaildropTarget", default)]
    taildrop_target: i32,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Cursor, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn file_put_url_escapes_target_and_filename_as_path_segments() {
        let url = file_put_url("node/id", "report #?.txt");

        assert_eq!(
            url.as_str(),
            "http://local-tailscaled.sock/localapi/v0/file-put/node%2Fid/report%20%23%3F.txt"
        );
    }

    #[test]
    fn push_file_streams_a_sized_put_over_the_unix_socket() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must be after the Unix epoch")
            .as_nanos();
        let socket_path = std::env::temp_dir().join(format!(
            "droptail-localapi-{}-{unique}.sock",
            std::process::id()
        ));
        let listener = UnixListener::bind(&socket_path).expect("test socket must be created");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("test request must connect");
            let mut request = Vec::new();
            let (header_end, content_length) = loop {
                let mut buffer = [0_u8; 1024];
                let read = stream.read(&mut buffer).expect("test request must be read");
                assert!(read > 0, "request ended before its headers");
                request.extend_from_slice(&buffer[..read]);

                let Some(header_end) = request
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .map(|index| index + 4)
                else {
                    continue;
                };
                let headers = std::str::from_utf8(&request[..header_end])
                    .expect("HTTP headers must be UTF-8");
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .expect("request must contain Content-Length");
                break (header_end, content_length);
            };

            while request.len() < header_end + content_length {
                let mut buffer = [0_u8; 1024];
                let read = stream.read(&mut buffer).expect("test body must be read");
                assert!(read > 0, "request ended before its body");
                request.extend_from_slice(&buffer[..read]);
            }

            let headers =
                std::str::from_utf8(&request[..header_end]).expect("HTTP headers must be UTF-8");
            assert!(
                headers
                    .starts_with("PUT /localapi/v0/file-put/node-1/notes%20%231.txt HTTP/1.1\r\n")
            );
            assert_eq!(&request[header_end..header_end + content_length], b"test");
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\n{}\n")
                .expect("test response must be written");
        });

        let client = LocalApiClient {
            client: Client::builder()
                .unix_socket(socket_path.clone())
                .build()
                .expect("test client must be built"),
        };
        client
            .push_file("node-1", "notes #1.txt", Some(4), Cursor::new(*b"test"))
            .expect("the LocalAPI PUT must succeed");

        server.join().expect("test server must finish");
        fs::remove_file(socket_path).expect("test socket must be removed");
    }

    #[test]
    fn push_file_streams_an_unknown_length_body_with_chunked_encoding() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must be after the Unix epoch")
            .as_nanos();
        let socket_path = std::env::temp_dir().join(format!(
            "droptail-localapi-stream-{}-{unique}.sock",
            std::process::id()
        ));
        let listener = UnixListener::bind(&socket_path).expect("test socket must be created");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("test request must connect");
            let mut request = Vec::new();
            let header_end = loop {
                let mut buffer = [0_u8; 1024];
                let read = stream.read(&mut buffer).expect("test request must be read");
                assert!(read > 0, "request ended before its headers");
                request.extend_from_slice(&buffer[..read]);
                if let Some(header_end) = request
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .map(|index| index + 4)
                {
                    break header_end;
                }
            };
            let headers =
                std::str::from_utf8(&request[..header_end]).expect("HTTP headers must be UTF-8");
            assert!(!headers.to_ascii_lowercase().contains("content-length:"));
            assert!(
                headers
                    .to_ascii_lowercase()
                    .contains("transfer-encoding: chunked")
            );

            while !request[header_end..].ends_with(b"0\r\n\r\n") {
                let mut buffer = [0_u8; 1024];
                let read = stream.read(&mut buffer).expect("test body must be read");
                assert!(read > 0, "request ended before its final chunk");
                request.extend_from_slice(&buffer[..read]);
            }
            let mut position = header_end;
            let mut body = Vec::new();
            loop {
                let line_end = request[position..]
                    .windows(2)
                    .position(|window| window == b"\r\n")
                    .map(|index| position + index)
                    .expect("chunk size must end with CRLF");
                let size = usize::from_str_radix(
                    std::str::from_utf8(&request[position..line_end])
                        .expect("chunk size must be ASCII"),
                    16,
                )
                .expect("chunk size must be hexadecimal");
                position = line_end + 2;
                if size == 0 {
                    break;
                }
                body.extend_from_slice(&request[position..position + size]);
                position += size + 2;
            }
            assert_eq!(body, b"streamed archive");
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\n{}\n")
                .expect("test response must be written");
        });

        let client = LocalApiClient {
            client: Client::builder()
                .unix_socket(socket_path.clone())
                .build()
                .expect("test client must be built"),
        };
        client
            .push_file(
                "node-1",
                "archive.tar.zst",
                None,
                Cursor::new(b"streamed archive"),
            )
            .expect("the streaming LocalAPI PUT must succeed");

        server.join().expect("test server must finish");
        fs::remove_file(socket_path).expect("test socket must be removed");
    }

    const STATUS: &str = r#"
    {
      "BackendState": "Running",
      "MagicDNSSuffix": "legacy.example.ts.net",
      "CurrentTailnet": { "MagicDNSSuffix": "example.ts.net" },
      "Peer": {
        "nodekey:direct": {
          "ID": "node-direct",
          "HostName": "direct-host",
          "DNSName": "desktop.example.ts.net.",
          "OS": "linux",
          "TailscaleIPs": ["fd7a:115c:a1e0::2", "100.64.0.2"],
          "CurAddr": "192.0.2.2:41641",
          "Relay": "fra",
          "Online": true,
          "TaildropTarget": 1
        },
        "nodekey:relay": {
          "ID": "node-relay",
          "HostName": "Pixel 9",
          "OS": "android",
          "TailscaleIPs": ["100.64.0.3"],
          "Relay": "fra",
          "Online": true,
          "TaildropTarget": 1
        },
        "nodekey:peer-relay": {
          "ID": "node-peer-relay",
          "DNSName": "studio.example.ts.net.",
          "OS": "windows",
          "TailscaleIPs": ["100.64.0.4"],
          "CurAddr": "192.0.2.4:41641",
          "Relay": "lhr",
          "PeerRelay": "100.64.0.10:1:vni:7",
          "Online": true,
          "TaildropTarget": 1
        },
        "nodekey:offline": {
          "ID": "node-offline",
          "DNSName": "iphone.example.ts.net.",
          "OS": "iOS",
          "TailscaleIPs": ["100.64.0.5"],
          "Online": false,
          "TaildropTarget": 5
        },
        "nodekey:unsupported": {
          "ID": "node-unsupported",
          "DNSName": "nas.example.ts.net.",
          "OS": "linux",
          "TailscaleIPs": ["100.64.0.6"],
          "Online": true,
          "TaildropTarget": 7
        }
      }
    }
    "#;

    #[test]
    fn status_maps_only_taildrop_targets_and_preserves_connection_details() {
        let devices = parse_devices(STATUS.as_bytes()).expect("fixture must be valid");
        assert_eq!(devices.len(), 4);

        let direct = device(&devices, "node-direct");
        assert_eq!(direct.name, "desktop");
        assert_eq!(direct.platform, "linux");
        assert_eq!(direct.address, "100.64.0.2".parse::<IpAddr>().unwrap());
        assert_eq!(direct.connection, Connection::Direct);

        let relay = device(&devices, "node-relay");
        assert_eq!(relay.name, "Pixel 9");
        assert_eq!(relay.platform, "android");
        assert_eq!(relay.connection, Connection::Derp("fra".to_owned()));

        let peer_relay = device(&devices, "node-peer-relay");
        assert_eq!(
            peer_relay.connection,
            Connection::PeerRelay("100.64.0.10:1:vni:7".to_owned())
        );

        let offline = device(&devices, "node-offline");
        assert_eq!(offline.connection, Connection::Offline);
        assert!(!devices.iter().any(|device| device.id == "node-unsupported"));
    }

    #[test]
    fn status_rejects_a_disconnected_backend() {
        let error = parse_devices(br#"{"BackendState":"Stopped","Peer":{}}"#)
            .expect_err("a stopped backend cannot provide send targets");
        assert!(matches!(error, LocalApiError::NotRunning { .. }));
    }

    fn device<'a>(devices: &'a [Device], id: &str) -> &'a Device {
        devices
            .iter()
            .find(|device| device.id == id)
            .expect("fixture device must exist")
    }
}
