use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;
use thiserror::Error;

use crate::domain::device::{Connection, Device};

const SOCKET_PATH: &str = "/var/run/tailscale/tailscaled.sock";
const STATUS_URL: &str = "http://local-tailscaled.sock/localapi/v0/status";
// Values from Tailscale's ipnstate.TaildropTargetStatus.
const TAILDROP_AVAILABLE: i32 = 1;
const TAILDROP_OFFLINE: i32 = 5;

#[derive(Debug, Error)]
pub enum LocalApiError {
    #[error("无法访问 tailscaled LocalAPI: {0}")]
    Request(#[from] reqwest::Error),
    #[error("tailscaled LocalAPI 返回了 HTTP {status}")]
    HttpStatus { status: u16 },
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

pub fn fetch_devices() -> Result<Vec<Device>, LocalApiError> {
    let body = get_status()?;
    parse_devices(&body)
}

fn get_status() -> Result<Vec<u8>, LocalApiError> {
    let response = Client::builder()
        .unix_socket(SOCKET_PATH)
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(5))
        .user_agent(concat!("droptail/", env!("CARGO_PKG_VERSION")))
        .build()?
        .get(STATUS_URL)
        .send()?;

    let status = response.status();
    if !status.is_success() {
        return Err(LocalApiError::HttpStatus {
            status: status.as_u16(),
        });
    }

    Ok(response.bytes()?.to_vec())
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
    use super::*;

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
