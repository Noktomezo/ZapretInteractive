use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

const JOURNAL_FILE: &str = "strategy-probe-session.json";
const REPORT_FILE: &str = "strategy-probe-report.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeSessionJournal {
    pub was_connected: bool,
}

fn journal_path(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join(JOURNAL_FILE)
}

pub fn report_path(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join(REPORT_FILE)
}

pub fn load_recovery_journal(runtime_dir: &Path) -> Result<Option<ProbeSessionJournal>> {
    let path = journal_path(runtime_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content)
            .with_context(|| format!("некорректный журнал {}", path.display()))
            .map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("не удалось прочитать {}", path.display()))
        }
    }
}

pub fn clear_recovery_journal(runtime_dir: &Path) -> Result<()> {
    crate::services::remove_if_exists(&journal_path(runtime_dir))
}

pub(super) fn write_journal(runtime_dir: &Path, was_connected: bool) -> Result<()> {
    write_json_replace(
        &journal_path(runtime_dir),
        &ProbeSessionJournal { was_connected },
    )
}

pub(super) fn write_json_replace(path: &Path, value: &impl Serialize) -> Result<()> {
    let temporary = path.with_extension("tmp");
    let payload = serde_json::to_vec_pretty(value).context("не удалось сериализовать JSON")?;
    std::fs::write(&temporary, payload)
        .with_context(|| format!("не удалось записать {}", temporary.display()))?;
    if path.is_file() {
        std::fs::remove_file(path)
            .with_context(|| format!("не удалось заменить {}", path.display()))?;
    }
    std::fs::rename(&temporary, path)
        .with_context(|| format!("не удалось заменить {}", path.display()))
}
