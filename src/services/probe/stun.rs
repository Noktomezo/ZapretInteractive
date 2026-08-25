use std::net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

use url::Url;

use crate::domain::{ProbeOutcome, ProbeProfile, ProbeProtocol, ProbeTarget, ProbeTargetResult};

const STUN_MAGIC_COOKIE: u32 = 0x2112A442;
const BINDING_REQUEST: u16 = 0x0001;
const BINDING_RESPONSE: u16 = 0x0101;

pub fn run_stun(
    target: &ProbeTarget,
    expected_protocol: ProbeProtocol,
    profile: &ProbeProfile,
) -> ProbeTargetResult {
    let started = Instant::now();
    let target_addr = match resolve_stun_target(target) {
        Ok(addr) => addr,
        Err(err) => {
            return stun_failure(
                target,
                expected_protocol,
                started.elapsed().as_millis(),
                &err,
            );
        }
    };

    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => socket,
        Err(err) => {
            return stun_failure(
                target,
                expected_protocol,
                started.elapsed().as_millis(),
                &format!("не удалось открыть UDP сокет: {err}"),
            );
        }
    };

    let timeout = Duration::from_millis(profile.timeout_ms.clamp(500, 30_000));
    if let Err(err) = socket.set_read_timeout(Some(timeout)) {
        return stun_failure(
            target,
            expected_protocol,
            started.elapsed().as_millis(),
            &format!("ошибка установки таймаута: {err}"),
        );
    }

    let mut request = [0u8; 20];
    request[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
    request[2..4].copy_from_slice(&0u16.to_be_bytes()); // Length 0
    request[4..8].copy_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());

    let transaction_id = uuid::Uuid::new_v4();
    let tx_bytes = transaction_id.as_bytes();
    request[8..20].copy_from_slice(&tx_bytes[0..12]);

    if let Err(err) = socket.send_to(&request, target_addr) {
        return stun_failure(
            target,
            expected_protocol,
            started.elapsed().as_millis(),
            &format!("ошибка отправки UDP пакета: {err}"),
        );
    }

    let mut response_buf = [0u8; 512];
    let (bytes_read, responder_addr) = match socket.recv_from(&mut response_buf) {
        Ok(res) => res,
        Err(err) => {
            let latency_ms = started.elapsed().as_millis();
            let msg = if err.kind() == std::io::ErrorKind::TimedOut
                || err.kind() == std::io::ErrorKind::WouldBlock
            {
                "таймаут ожидания STUN ответа (пакет заблокирован)".to_string()
            } else {
                format!("ошибка получения UDP ответа: {err}")
            };
            return stun_failure(target, expected_protocol, latency_ms, &msg);
        }
    };

    let latency_ms = started.elapsed().as_millis();

    if bytes_read < 20 {
        return ProbeTargetResult {
            target_id: target.id.clone(),
            target_name: target.name.clone(),
            target_url: target.url.clone(),
            expected_protocol,
            outcome: ProbeOutcome::Degraded,
            protocol: Some("STUN".to_string()),
            status_code: None,
            bytes: bytes_read as u64,
            remote_ip: Some(responder_addr.ip().to_string()),
            latency_ms,
            error: Some(format!("ответ слишком короткий ({bytes_read} байт)")),
        };
    }

    let msg_type = u16::from_be_bytes([response_buf[0], response_buf[1]]);
    let magic_cookie = u32::from_be_bytes([
        response_buf[4],
        response_buf[5],
        response_buf[6],
        response_buf[7],
    ]);
    let resp_tx_id = &response_buf[8..20];

    if magic_cookie != STUN_MAGIC_COOKIE || resp_tx_id != &request[8..20] {
        return ProbeTargetResult {
            target_id: target.id.clone(),
            target_name: target.name.clone(),
            target_url: target.url.clone(),
            expected_protocol,
            outcome: ProbeOutcome::Degraded,
            protocol: Some("STUN".to_string()),
            status_code: Some(msg_type),
            bytes: bytes_read as u64,
            remote_ip: Some(responder_addr.ip().to_string()),
            latency_ms,
            error: Some("несовпадение STUN transaction ID или magic cookie".to_string()),
        };
    }

    let outcome = if msg_type == BINDING_RESPONSE {
        ProbeOutcome::Pass
    } else {
        ProbeOutcome::Degraded
    };

    ProbeTargetResult {
        target_id: target.id.clone(),
        target_name: target.name.clone(),
        target_url: target.url.clone(),
        expected_protocol,
        outcome,
        protocol: Some("STUN".to_string()),
        status_code: Some(msg_type),
        bytes: bytes_read as u64,
        remote_ip: Some(responder_addr.ip().to_string()),
        latency_ms,
        error: if outcome == ProbeOutcome::Pass {
            None
        } else {
            Some(format!("получен STUN код ответа 0x{msg_type:04X}"))
        },
    }
}

fn resolve_stun_target(target: &ProbeTarget) -> Result<SocketAddr, String> {
    if let Some(connect_ip) = &target.connect_ip {
        let ip: IpAddr = connect_ip
            .parse()
            .map_err(|e| format!("некорректный connect_ip: {e}"))?;
        let port = extract_port(&target.url).unwrap_or(19302);
        return Ok(SocketAddr::new(ip, port));
    }

    let url = Url::parse(&target.url).map_err(|e| format!("некорректный URL: {e}"))?;
    let host = url
        .host_str()
        .ok_or_else(|| "отсутствует хост в URL".to_string())?;
    let port = url.port().unwrap_or(19302);

    let addrs = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("ошибка DNS резолва {host}:{port}: {e}"))?;

    addrs
        .into_iter()
        .find(|addr| addr.is_ipv4())
        .ok_or_else(|| format!("не найден IPv4 адрес для {host}"))
}

fn extract_port(url_str: &str) -> Option<u16> {
    Url::parse(url_str).ok().and_then(|u| u.port())
}

fn stun_failure(
    target: &ProbeTarget,
    expected_protocol: ProbeProtocol,
    latency_ms: u128,
    error: &str,
) -> ProbeTargetResult {
    ProbeTargetResult {
        target_id: target.id.clone(),
        target_name: target.name.clone(),
        target_url: target.url.clone(),
        expected_protocol,
        outcome: ProbeOutcome::Fail,
        protocol: Some("STUN".to_string()),
        status_code: None,
        bytes: 0,
        remote_ip: None,
        latency_ms,
        error: Some(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stun_request_format() {
        let mut request = [0u8; 20];
        request[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
        request[2..4].copy_from_slice(&0u16.to_be_bytes());
        request[4..8].copy_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
        assert_eq!(request[0], 0x00);
        assert_eq!(request[1], 0x01);
        assert_eq!(request[4..8], [0x21, 0x12, 0xA4, 0x42]);
    }
}
