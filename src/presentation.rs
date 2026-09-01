use crate::domain::content::SendMethod;
use crate::domain::device::Connection;

pub fn format_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * KIB;
    const GIB: f64 = 1024.0 * MIB;

    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.2} GiB", bytes / GIB)
    } else if bytes >= MIB {
        let mib = bytes / MIB;
        if mib >= 100.0 {
            format!("{mib:.0} MiB")
        } else {
            format!("{mib:.1} MiB")
        }
    } else {
        format!("{:.1} KiB", bytes / KIB)
    }
}

pub fn format_rate(bytes_per_second: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    format!("{:.1} MiB/s", bytes_per_second as f64 / MIB)
}

pub fn send_method_label(method: SendMethod) -> &'static str {
    match method {
        SendMethod::Separate => "分别发送",
        SendMethod::Archive => "打包后发送",
    }
}

pub fn connection_label(connection: &Connection) -> String {
    match connection {
        Connection::Offline => "离线".to_owned(),
        Connection::Direct => "直连".to_owned(),
        Connection::Derp(region) => format!("DERP · {region}"),
        Connection::PeerRelay(_) => "Peer Relay".to_owned(),
    }
}
