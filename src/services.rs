pub mod async_runtime;
pub mod binaries;
mod discord;
pub mod dns;
pub mod file_watcher;
pub mod process;
pub mod single_instance;
pub mod updater;

pub use async_runtime::{run_tokio, spawn_tokio};
pub use process::{RuntimeServices, cleanup_orphaned_processes, is_elevated};

pub(crate) fn remove_if_exists(path: &std::path::Path) -> anyhow::Result<()> {
    use anyhow::Context as _;

    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("не удалось удалить {}", path.display())),
    }
}

#[cfg(windows)]
pub(crate) fn hidden_cmd<T, U>(program: T, args: U) -> duct::Expression
where
    T: duct::IntoExecutablePath,
    U: IntoIterator,
    U::Item: Into<std::ffi::OsString>,
{
    use std::os::windows::process::CommandExt as _;
    duct::cmd(program, args).before_spawn(|command| {
        command.creation_flags(0x0800_0000);
        Ok(())
    })
}

#[cfg(not(windows))]
pub(crate) fn hidden_cmd<T, U>(program: T, args: U) -> duct::Expression
where
    T: duct::IntoExecutablePath,
    U: IntoIterator,
    U::Item: Into<std::ffi::OsString>,
{
    duct::cmd(program, args)
}
