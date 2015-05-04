use std::path::PathBuf;
use std::str;

#[cfg(not(windows))]
pub fn open_file() -> Option<PathBuf> {
    use std::env;
    use std::process::Command;

    let out = Command::new("kdialog").arg("--getopenurl").arg(env::current_dir().unwrap())
                                     .arg("*").output();
    let out = match out {
        Ok(out) => out,
        Err(e) => {
            println!("couldn't open dialog: {:?}", e);
            return None;
        }
    };

    if !out.status.success() {
        return None;
    }

    let file = str::from_utf8(&out.stdout).unwrap().trim();
    Some(PathBuf::from(regex!("^file://").replace(file, "")))
}

#[cfg(windows)]
mod comdlg32 {
    extern crate winapi;
    pub use self::winapi::*;

    shared_library!(ComDlg32, "comdlg32", pub fn GetOpenFileNameA(_ofn: *mut OPENFILENAME) -> BOOL,);

    pub type LPCTSTR = LPCSTR;
    pub type LPTSTR = LPSTR;

    #[repr(C)]
    #[allow(non_snake_case)]
    pub struct OPENFILENAME {
        pub lStructSize: DWORD,
        pub hwndOwner: HWND,
        pub hInstance: HINSTANCE,
        pub lpstrFilter: LPCTSTR,
        pub lpstrCustomFilter: LPTSTR,
        pub nMaxCustFilter: DWORD,
        pub nFilterIndex: DWORD,
        pub lpstrFile: LPTSTR,
        pub nMaxFile: DWORD,
        pub lpstrFileTitle: LPTSTR,
        pub nMaxFileTitle: DWORD,
        pub lpstrInitialDir: LPCTSTR,
        pub lpstrTitle: LPCTSTR,
        pub Flags: DWORD,
        pub nFileOffset: WORD,
        pub nFileExtension: WORD,
        pub lpstrDefExt: LPCTSTR,
        pub lCustData: LPARAM,
        pub lpfnHook: *const (),//LPOFNHOOKPROC,
        pub lpTemplateName: LPCTSTR,
        pub pvReserved: *const (),
        pub dwReserved: DWORD,
        pub FlagsEx: DWORD
    }

    pub fn get_GetOpenFileNameA() -> unsafe extern "system" fn(*mut OPENFILENAME) -> BOOL {
        use std::mem;
        unsafe {
            mem::transmute(ComDlg32::get_static_ref().GetOpenFileNameA)
        }
    }
}

#[cfg(windows)]
pub fn open_file() -> Option<PathBuf> {
    use self::comdlg32::*;
    use std::ffi::CStr;
    use std::mem;

    let mut file = [0u8; 260];

    let mut ofn: OPENFILENAME = unsafe { mem::zeroed() };
    ofn.lStructSize = mem::size_of::<OPENFILENAME>() as DWORD;

    ofn.lpstrFile = file.as_mut_ptr() as LPTSTR;
    ofn.nMaxFile = 260;
    ofn.lpstrFilter = b"All\0*.*\0Rust code\0*.RS\0\0".as_ptr() as LPCTSTR;
    ofn.nFilterIndex = 1;
    ofn.Flags = 0x00000800 | 0x00001000; // OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST;

    if unsafe { get_GetOpenFileNameA()(&mut ofn) } == TRUE {
        Some(PathBuf::from(str::from_utf8(unsafe {
            CStr::from_ptr(ofn.lpstrFile).to_bytes()
        }).unwrap()))
    } else {
        None
    }
}
