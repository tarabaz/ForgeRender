//! Funzioni dell'API C di OIDN 2.x usate da ForgeRender (vedi `oidn.h`).

use libloading::Library;
use std::ffi::{c_char, c_void};
use std::path::Path;

pub type OidnDevice = *mut c_void;
pub type OidnFilter = *mut c_void;
pub type OidnBuffer = *mut c_void;

pub struct Api {
    _lib: Library,
    pub new_device: unsafe extern "C" fn(i32) -> OidnDevice,
    pub commit_device: unsafe extern "C" fn(OidnDevice),
    pub release_device: unsafe extern "C" fn(OidnDevice),
    pub get_device_error: unsafe extern "C" fn(OidnDevice, *mut *const c_char) -> i32,
    pub get_device_int: unsafe extern "C" fn(OidnDevice, *const c_char) -> i32,
    pub new_buffer: unsafe extern "C" fn(OidnDevice, usize) -> OidnBuffer,
    pub write_buffer: unsafe extern "C" fn(OidnBuffer, usize, usize, *const c_void),
    pub read_buffer: unsafe extern "C" fn(OidnBuffer, usize, usize, *mut c_void),
    pub release_buffer: unsafe extern "C" fn(OidnBuffer),
    pub new_filter: unsafe extern "C" fn(OidnDevice, *const c_char) -> OidnFilter,
    #[allow(clippy::type_complexity)]
    pub set_filter_image: unsafe extern "C" fn(OidnFilter, *const c_char, OidnBuffer, i32, usize, usize, usize, usize, usize),
    pub set_filter_bool: unsafe extern "C" fn(OidnFilter, *const c_char, bool),
    pub set_filter_int: unsafe extern "C" fn(OidnFilter, *const c_char, i32),
    pub commit_filter: unsafe extern "C" fn(OidnFilter),
    pub execute_filter: unsafe extern "C" fn(OidnFilter),
    pub release_filter: unsafe extern "C" fn(OidnFilter),
}

#[cfg(windows)]
unsafe fn open(path: &Path) -> Result<Library, libloading::Error> {
    // Con un percorso assoluto, le dipendenze (OpenImageDenoise_core.dll, tbb12.dll) vanno
    // cercate nella cartella della DLL stessa, non solo in quella dell'eseguibile.
    const LOAD_WITH_ALTERED_SEARCH_PATH: u32 = 0x0000_0008;
    let flags = if path.is_absolute() { LOAD_WITH_ALTERED_SEARCH_PATH } else { 0 };
    libloading::os::windows::Library::load_with_flags(path, flags).map(Library::from)
}

#[cfg(not(windows))]
unsafe fn open(path: &Path) -> Result<Library, libloading::Error> {
    Library::new(path)
}

impl Api {
    /// # Safety
    /// `path` deve essere la libreria OIDN 2.x: le firme qui sotto devono corrispondere.
    pub unsafe fn load(path: &Path) -> Result<Self, libloading::Error> {
        let lib = open(path)?;
        macro_rules! sym {
            ($name:literal) => {
                *lib.get(concat!($name, "\0").as_bytes())?
            };
        }
        Ok(Self {
            new_device: sym!("oidnNewDevice"),
            commit_device: sym!("oidnCommitDevice"),
            release_device: sym!("oidnReleaseDevice"),
            get_device_error: sym!("oidnGetDeviceError"),
            get_device_int: sym!("oidnGetDeviceInt"),
            new_buffer: sym!("oidnNewBuffer"),
            write_buffer: sym!("oidnWriteBuffer"),
            read_buffer: sym!("oidnReadBuffer"),
            release_buffer: sym!("oidnReleaseBuffer"),
            new_filter: sym!("oidnNewFilter"),
            set_filter_image: sym!("oidnSetFilterImage"),
            set_filter_bool: sym!("oidnSetFilterBool"),
            set_filter_int: sym!("oidnSetFilterInt"),
            commit_filter: sym!("oidnCommitFilter"),
            execute_filter: sym!("oidnExecuteFilter"),
            release_filter: sym!("oidnReleaseFilter"),
            _lib: lib,
        })
    }
}
