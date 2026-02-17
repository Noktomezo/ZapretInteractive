#[cfg(windows)]
use windows::Win32::Foundation::HANDLE;
#[cfg(windows)]
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
#[cfg(windows)]
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

#[tauri::command]
pub fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        unsafe {
            let mut token_handle = HANDLE::default();
            let process = GetCurrentProcess();

            if OpenProcessToken(process, TOKEN_QUERY, &mut token_handle).is_err() {
                return false;
            }

            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut size = 0u32;

            let result = GetTokenInformation(
                token_handle,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            );

            let _ = windows::Win32::Foundation::CloseHandle(token_handle);

            result.is_ok() && elevation.TokenIsElevated != 0
        }
    }

    #[cfg(not(windows))]
    {
        true
    }
}
