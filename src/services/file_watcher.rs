use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context as _, Result};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

const WATCH_DEBOUNCE: Duration = Duration::from_millis(800);

#[derive(Debug)]
pub enum FileWatchEvent {
    Changed,
    Error(String),
}

pub fn start(resources_dir: &Path) -> Result<UnboundedReceiver<FileWatchEvent>> {
    let (notify_tx, notify_rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |event| {
            let _receiver_closed = notify_tx.send(event);
        },
        Config::default(),
    )
    .context("не удалось запустить watcher управляемых файлов")?;
    watcher
        .watch(resources_dir, RecursiveMode::Recursive)
        .with_context(|| format!("не удалось наблюдать за {}", resources_dir.display()))?;

    let resources_dir = resources_dir.to_path_buf();
    let (event_tx, event_rx) = unbounded_channel();
    std::thread::Builder::new()
        .name("managed-files-watcher".to_owned())
        .spawn(move || {
            let _watcher = watcher;
            while let Ok(first) = notify_rx.recv() {
                let mut batch = vec![first];
                while let Ok(next) = notify_rx.recv_timeout(WATCH_DEBOUNCE) {
                    batch.push(next);
                }

                let mut changed_paths = Vec::new();
                for event in batch {
                    match event {
                        Ok(event) if !matches!(event.kind, EventKind::Access(_)) => {
                            changed_paths.extend(event.paths);
                        }
                        Ok(_) => {}
                        Err(error) => {
                            if event_tx
                                .send(FileWatchEvent::Error(format!(
                                    "Ошибка watcher управляемых файлов: {error}"
                                )))
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                }

                if changed_paths
                    .iter()
                    .any(|path| is_managed_path(&resources_dir, path))
                    && event_tx.send(FileWatchEvent::Changed).is_err()
                {
                    return;
                }
            }
        })
        .context("не удалось создать поток watcher управляемых файлов")?;

    Ok(event_rx)
}

fn is_managed_path(resources_dir: &Path, path: &Path) -> bool {
    if path == resources_dir {
        return true;
    }
    let Ok(relative) = path.strip_prefix(resources_dir) else {
        return false;
    };
    let normalized = relative.to_string_lossy().replace('\\', "/");
    normalized == "hashes.json"
        || matches!(
            normalized.as_str(),
            "WinDivert.dll" | "Monkey64.sys" | "winws.exe" | "cygwin1.dll"
        )
        || ["fake/", "lists/", "modules/", "filters/"]
            .iter()
            .any(|prefix| normalized.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn only_managed_resource_changes_trigger_repairs() {
        let root = PathBuf::from("C:/app/thirdparty");
        assert!(is_managed_path(&root, &root.join("lists/custom.txt")));
        assert!(is_managed_path(
            &root,
            &root.join("modules/dnscrypt-proxy/dnscrypt-proxy.exe.tmp")
        ));
        assert!(is_managed_path(&root, &root.join("hashes.json")));
        assert!(!is_managed_path(&root, &root.join("strategies/http.json")));
        assert!(!is_managed_path(
            &root,
            &PathBuf::from("C:/elsewhere/winws.exe")
        ));
    }
}
