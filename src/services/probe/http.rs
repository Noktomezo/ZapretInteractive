use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::domain::{
    ProbeOutcome, ProbeProfile, ProbeProtocol, ProbeRole, ProbeTarget, ProbeTargetResult,
};
use crate::services::hidden_cmd;

pub(super) fn run_targets(
    curl: &Path,
    profile: &ProbeProfile,
    full: bool,
    cancelled: &AtomicBool,
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
    for chunk in jobs.chunks(profile.parallel_targets) {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        std::thread::scope(|scope| {
            let handles = chunk
                .iter()
                .map(|(target, protocol)| {
                    scope.spawn(move || run_curl(curl, target, *protocol, profile.timeout_ms))
                })
                .collect::<Vec<_>>();
            for handle in handles {
                results.push(match handle.join() {
                    Ok(result) => result,
                    Err(_) => failed_result("probe worker panicked"),
                });
            }
        });
    }
    results
}

fn run_curl(
    curl: &Path,
    target: &ProbeTarget,
    expected_protocol: ProbeProtocol,
    timeout_ms: u64,
) -> ProbeTargetResult {
    let started = Instant::now();
    let timeout_seconds = timeout_ms.div_ceil(1_000).max(1).to_string();
    let protocol_flag = match expected_protocol {
        ProbeProtocol::Http11 => "--http1.1",
        ProbeProtocol::Http2 => "--http2",
        ProbeProtocol::Http3 => "--http3-only",
    };
    let args = [
        "--silent",
        "--show-error",
        "--no-progress-meter",
        "--impersonate",
        "chrome145",
        protocol_flag,
        "--range",
        "0-65535",
        "--connect-timeout",
        &timeout_seconds,
        "--max-time",
        &timeout_seconds,
        "--max-redirs",
        "0",
        "--output",
        "NUL",
        "--write-out",
        "%{http_code}\t%{http_version}\t%{size_download}",
        target.url.as_str(),
    ];
    let output = hidden_cmd(curl, args)
        .unchecked()
        .stdout_capture()
        .stderr_capture()
        .run();
    let latency_ms = started.elapsed().as_millis();
    let Ok(output) = output else {
        return target_failure(target, latency_ms, "не удалось запустить curl-impersonate");
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut fields = stdout.trim().rsplitn(3, '\t').collect::<Vec<_>>();
    fields.reverse();
    if !output.status.success() || fields.len() != 3 {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return target_failure(
            target,
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
    let intended_protocol = match expected_protocol {
        ProbeProtocol::Http11 => protocol == "1.1",
        ProbeProtocol::Http2 => protocol == "2",
        ProbeProtocol::Http3 => protocol == "3",
    };
    let outcome = match status_code {
        Some(200..=499) if intended_protocol => ProbeOutcome::Pass,
        Some(500..=599) if intended_protocol => ProbeOutcome::Degraded,
        _ => ProbeOutcome::Fail,
    };
    ProbeTargetResult {
        target_id: target.id.clone(),
        role: target.role,
        outcome,
        protocol: Some(protocol),
        status_code,
        bytes,
        latency_ms,
        error: None,
    }
}

fn failed_result(message: &str) -> ProbeTargetResult {
    ProbeTargetResult {
        target_id: "worker".to_owned(),
        role: ProbeRole::Required,
        outcome: ProbeOutcome::Fail,
        protocol: None,
        status_code: None,
        bytes: 0,
        latency_ms: 0,
        error: Some(message.to_owned()),
    }
}

fn target_failure(target: &ProbeTarget, latency_ms: u128, message: &str) -> ProbeTargetResult {
    ProbeTargetResult {
        target_id: target.id.clone(),
        role: target.role,
        outcome: ProbeOutcome::Fail,
        protocol: None,
        status_code: None,
        bytes: 0,
        latency_ms,
        error: Some(message.to_owned()),
    }
}
