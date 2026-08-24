use std::net::ToSocketAddrs as _;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::domain::{ProbeOutcome, ProbeProfile, ProbeProtocol, ProbeTarget, ProbeTargetResult};
use crate::services::hidden_cmd;

const GGC_REDIRECTORS: [&str; 4] = [
    "gvt1.com",
    "c.youtube.com",
    "c.googlevideo.com",
    "googlevideo.com",
];

pub(super) fn run_targets(
    curl: &Path,
    profile: &ProbeProfile,
    full: bool,
    cancelled: &AtomicBool,
    on_results: &impl Fn(&[ProbeTargetResult]),
) -> Vec<ProbeTargetResult> {
    let jobs = profile
        .targets_for(full)
        .flat_map(|target| {
            profile
                .protocols
                .iter()
                .copied()
                .map(move |protocol| (target, protocol))
        })
        .collect::<Vec<_>>();
    let mut results = Vec::with_capacity(jobs.len());
    on_results(&results);
    for chunk in jobs.chunks(profile.parallel_targets) {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        std::thread::scope(|scope| {
            let handles = chunk
                .iter()
                .map(|(target, protocol)| {
                    scope.spawn(move || run_curl(curl, target, *protocol, profile))
                })
                .collect::<Vec<_>>();
            for handle in handles {
                results.push(match handle.join() {
                    Ok(result) => result,
                    Err(_) => failed_result("probe worker panicked"),
                });
                on_results(&results);
            }
        });
    }
    results
}

pub(super) fn discover_youtube_ggc(curl: &Path, profile: &ProbeProfile) -> Option<ProbeTarget> {
    let timeout_seconds = profile.timeout_ms.div_ceil(1_000).max(1).to_string();
    let encoded = GGC_REDIRECTORS.iter().find_map(|domain| {
        let mut args = vec![
            "--silent".to_owned(),
            "--show-error".to_owned(),
            "--compressed".to_owned(),
            "--ipv4".to_owned(),
            "--impersonate".to_owned(),
            profile.impersonate.as_curl_target().to_owned(),
            "--ech".to_owned(),
            "false".to_owned(),
            "--max-time".to_owned(),
            timeout_seconds.clone(),
        ];
        if let Some(doh_url) = &profile.doh_url {
            args.extend(["--doh-url".to_owned(), doh_url.clone()]);
        }
        args.push(format!("https://redirector.{domain}/report_mapping?di=no"));
        let output = hidden_cmd(curl, args)
            .unchecked()
            .stdout_capture()
            .stderr_capture()
            .run()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .nth(2)
            .map(str::to_owned)
    })?;
    let suffix = decode_ggc_suffix(&encoded)?;
    (1..=12).find_map(|index| {
        let host = format!("rr{index}---sn-{suffix}.googlevideo.com");
        let connect_ip = (host.as_str(), 443)
            .to_socket_addrs()
            .ok()?
            .find(|address| address.is_ipv4())?
            .ip()
            .to_string();
        Some(ProbeTarget {
            id: "youtube-local-ggc".to_owned(),
            name: "Local YouTube GGC".to_owned(),
            url: format!("https://{host}/generate_204"),
            _legacy_role: None,
            tier: crate::domain::ProbeTier::Full,
            min_bytes: 0,
            connect_ip: Some(connect_ip),
        })
    })
}

fn decode_ggc_suffix(encoded: &str) -> Option<String> {
    const SOURCE: &str = "uzpkfa50vqlgb61wrmhc72xsnid83ytoje94-_";
    const DECODED: &str = "0123456789abcdefghijklmnopqrstuvwxyz-_";
    encoded
        .chars()
        .map(|character| {
            SOURCE
                .chars()
                .position(|source| source == character)
                .and_then(|index| DECODED.chars().nth(index))
        })
        .collect()
}

fn run_curl(
    curl: &Path,
    target: &ProbeTarget,
    expected_protocol: ProbeProtocol,
    profile: &ProbeProfile,
) -> ProbeTargetResult {
    let started = Instant::now();
    let timeout_seconds = profile.timeout_ms.div_ceil(1_000).max(1).to_string();
    let protocol_flag = match expected_protocol {
        ProbeProtocol::Auto => None,
        ProbeProtocol::Http11 => Some("--http1.1"),
        ProbeProtocol::Http2 => Some("--http2"),
        ProbeProtocol::Http3 => Some("--http3-only"),
    };
    let range = format!("0-{}", profile.download_bytes.saturating_sub(1));
    let mut args = vec![
        "--silent".to_owned(),
        "--show-error".to_owned(),
        "--no-progress-meter".to_owned(),
        "--compressed".to_owned(),
        "--ipv4".to_owned(),
        "--impersonate".to_owned(),
        profile.impersonate.as_curl_target().to_owned(),
        "--ech".to_owned(),
        "false".to_owned(),
    ];
    if let Some(protocol_flag) = protocol_flag {
        args.push(protocol_flag.to_owned());
    }
    args.extend([
        "--range".to_owned(),
        range,
        "--connect-timeout".to_owned(),
        timeout_seconds.clone(),
        "--max-time".to_owned(),
        timeout_seconds,
        "--max-redirs".to_owned(),
        if profile.follow_redirects { "5" } else { "0" }.to_owned(),
    ]);
    if profile.follow_redirects {
        args.push("--location".to_owned());
    }
    if let Some(doh_url) = &profile.doh_url {
        args.extend(["--doh-url".to_owned(), doh_url.clone()]);
    }
    if let Some(connect_ip) = &target.connect_ip
        && let Ok(url) = url::Url::parse(&target.url)
        && let Some(host) = url.host_str()
    {
        let port = url.port_or_known_default().unwrap_or(443);
        args.extend([
            "--resolve".to_owned(),
            format!("{host}:{port}:{connect_ip}"),
        ]);
    }
    args.extend([
        "--output".to_owned(),
        "NUL".to_owned(),
        "--write-out".to_owned(),
        "%{http_code}\t%{http_version}\t%{size_download}\t%{remote_ip}".to_owned(),
        target.url.clone(),
    ]);
    let output = hidden_cmd(curl, args)
        .unchecked()
        .stdout_capture()
        .stderr_capture()
        .run();
    let latency_ms = started.elapsed().as_millis();
    let Ok(output) = output else {
        return target_failure(
            target,
            expected_protocol,
            latency_ms,
            "не удалось запустить curl-impersonate",
        );
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut fields = stdout.trim().rsplitn(4, '\t').collect::<Vec<_>>();
    fields.reverse();
    if !output.status.success() || fields.len() != 4 {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return target_failure(
            target,
            expected_protocol,
            latency_ms,
            if error.is_empty() {
                "curl завершился с ошибкой"
            } else {
                &error
            },
        );
    }
    let status_code = fields[0].parse::<u16>().ok();
    let protocol = fields[1].to_owned();
    let bytes = fields[2].parse::<u64>().unwrap_or(0);
    let remote_ip = (!fields[3].is_empty()).then(|| fields[3].to_owned());
    let intended_protocol = match expected_protocol {
        ProbeProtocol::Auto => matches!(protocol.as_str(), "1.1" | "2" | "3"),
        ProbeProtocol::Http11 => protocol == "1.1",
        ProbeProtocol::Http2 => protocol == "2",
        ProbeProtocol::Http3 => protocol == "3",
    };
    let enough_bytes = bytes >= target.min_bytes;
    let outcome = match status_code {
        Some(200..=399) if intended_protocol && enough_bytes => ProbeOutcome::Pass,
        Some(400..=599) if intended_protocol => ProbeOutcome::Degraded,
        _ => ProbeOutcome::Fail,
    };
    ProbeTargetResult {
        target_id: target.id.clone(),
        target_name: target.name.clone(),
        target_url: target.url.clone(),
        expected_protocol,
        outcome,
        protocol: Some(protocol),
        status_code,
        bytes,
        remote_ip,
        latency_ms,
        error: (!enough_bytes).then(|| {
            format!(
                "получено {bytes} байт, требуется не менее {}",
                target.min_bytes
            )
        }),
    }
}

fn failed_result(message: &str) -> ProbeTargetResult {
    ProbeTargetResult {
        target_id: "worker".to_owned(),
        target_name: "Worker".to_owned(),
        target_url: String::new(),
        expected_protocol: ProbeProtocol::Auto,
        outcome: ProbeOutcome::Fail,
        protocol: None,
        status_code: None,
        bytes: 0,
        remote_ip: None,
        latency_ms: 0,
        error: Some(message.to_owned()),
    }
}

fn target_failure(
    target: &ProbeTarget,
    expected_protocol: ProbeProtocol,
    latency_ms: u128,
    message: &str,
) -> ProbeTargetResult {
    ProbeTargetResult {
        target_id: target.id.clone(),
        target_name: target.name.clone(),
        target_url: target.url.clone(),
        expected_protocol,
        outcome: ProbeOutcome::Fail,
        protocol: None,
        status_code: None,
        bytes: 0,
        remote_ip: None,
        latency_ms,
        error: Some(message.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::decode_ggc_suffix;

    #[test]
    fn decodes_redirector_cache_suffix() {
        assert_eq!(decode_ggc_suffix("pnmila-an").as_deref(), Some("2ohpa5-5o"));
    }
}
