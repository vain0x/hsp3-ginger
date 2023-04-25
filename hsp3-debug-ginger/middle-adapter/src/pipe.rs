use std::{
    fs::File,
    io::{self, Read, Write},
    mem::size_of,
    os::windows::{
        io::InvalidHandleError,
        prelude::{HandleOrInvalid, OwnedHandle, RawHandle},
    },
    ptr::null_mut,
};
use winapi::{
    shared::minwindef::{DWORD, TRUE},
    um::{
        errhandlingapi::GetLastError,
        minwinbase::SECURITY_ATTRIBUTES,
        winbase::{PIPE_ACCESS_DUPLEX, PIPE_READMODE_MESSAGE, PIPE_TYPE_MESSAGE},
    },
};

pub(crate) struct Pipe(pub(crate) File);

/// 4MB
const BUFFER_SIZE: u32 = 4 * 1024 * 1024;

// 10sec
const TIMEOUT: u32 = 10 * 1000;

impl Pipe {
    /// `name` must be in the form of `\\.\pipe\somename`.
    pub(crate) fn new(name: &str) -> Self {
        let mut name_buf: Vec<u8> = Vec::with_capacity(260);
        // name_buf.extend_from_slice(r#"\\.\pipe\"#.as_bytes());
        name_buf.extend_from_slice(name.as_bytes());
        name_buf.push(0);

        let mut sa = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as DWORD,
            lpSecurityDescriptor: null_mut(),
            bInheritHandle: TRUE,
        };

        let h = unsafe {
            winapi::um::winbase::CreateNamedPipeA(
                name_buf.as_mut_ptr() as *const i8,
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE,
                1,
                BUFFER_SIZE,
                BUFFER_SIZE,
                TIMEOUT,
                &mut sa as *mut SECURITY_ATTRIBUTES,
            )
        } as RawHandle;

        let h: OwnedHandle = {
            // Reject INVALID_HANDLE_VALUE
            let h = unsafe { HandleOrInvalid::from_raw_handle(h) };
            OwnedHandle::try_from(h).unwrap_or_else(|_: InvalidHandleError| {
                let n = unsafe { GetLastError() };
                panic!("CreateNamedPipeA {n}")
            })
        };

        let file = File::from(h);
        Pipe(file)
    }

    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        self.0.try_clone().map(Pipe)
    }
}

impl Read for Pipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Read::read(&mut self.0, buf)
    }
}

impl Write for Pipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Write::write(&mut self.0, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Write::flush(&mut self.0)
    }
}
