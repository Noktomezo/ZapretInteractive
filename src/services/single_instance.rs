#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
#[cfg(windows)]
use windows::Win32::System::Threading::CreateMutexW;
#[cfg(windows)]
use windows::core::w;

pub struct SingleInstanceGuard {
    #[cfg(windows)]
    handle: Option<HANDLE>,
}

impl SingleInstanceGuard {
    pub fn acquire() -> Option<Self> {
        #[cfg(windows)]
        {
            #[cfg(debug_assertions)]
            let mutex_name = w!("Local\\ZapretInteractive_Dev_SingleInstance_Mutex");
            #[cfg(not(debug_assertions))]
            let mutex_name = w!("Local\\ZapretInteractive_SingleInstance_Mutex");

            // SAFETY: Calling Win32 CreateMutexW with a static name.
            let handle = unsafe { CreateMutexW(None, true, mutex_name) };
            match handle {
                Ok(h) => {
                    // SAFETY: Checking GetLastError() immediately after CreateMutexW.
                    let error = unsafe { GetLastError() };
                    if error == ERROR_ALREADY_EXISTS {
                        // SAFETY: Closing handle on duplicate instance.
                        let _close_result = unsafe { CloseHandle(h) };
                        None
                    } else {
                        Some(Self { handle: Some(h) })
                    }
                }
                Err(_) => Some(Self { handle: None }),
            }
        }
        #[cfg(not(windows))]
        {
            Some(Self {})
        }
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            if let Some(handle) = self.handle.take() {
                // SAFETY: Closing mutex handle on app shutdown.
                let _close_result = unsafe { CloseHandle(handle) };
            }
        }
    }
}
