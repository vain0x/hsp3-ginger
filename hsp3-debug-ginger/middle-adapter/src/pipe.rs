#![cfg(target_os = "windows")]

// ドキュメント: [CreateNamedPipeA 関数 (winbase.h)](https://learn.microsoft.com/ja-jp/windows/win32/api/winbase/nf-winbase-createnamedpipea) ほか
// 参照: [Win32API 名前付きパイプによるプロセス間通信 CreateNamedPipe - s-kita’s blog](https://s-kita.hatenablog.com/entry/20100216/1266318458)

use std::{
    fs::File,
    io::{self, Read, Write},
    os::windows::{
        io::InvalidHandleError,
        prelude::{AsRawHandle, HandleOrInvalid, OwnedHandle, RawHandle},
    },
    ptr::null_mut,
};
use winapi::{
    shared::ntdef::HANDLE,
    um::{
        errhandlingapi::GetLastError,
        namedpipeapi::ConnectNamedPipe,
        winbase::{CreateNamedPipeA, PIPE_ACCESS_DUPLEX, PIPE_READMODE_MESSAGE, PIPE_TYPE_MESSAGE},
    },
};

/// 名前付きパイプ
///
/// パイプは `Read`, `Write` トレイトを実装しているため、
/// ファイル (`std::fs::File`) と同様に読み書きができる。
/// すなわち、`read` メソッドによる読み取りや `write!()` マクロによる書き込みができる
pub(crate) struct Pipe {
    f: File,
}

// unsafe impl Send for Pipe {}
// unsafe impl Sync for Pipe {}

/// 4MB
const BUFFER_SIZE: u32 = 4 * 1024 * 1024;

/// 10秒
const TIMEOUT: u32 = 10 * 1000;

impl Pipe {
    /// 名前付きパイプを生成する
    ///
    /// (Windowsの制約として) 名前は `\\.\pipe\name` という形式でなければいけない
    ///
    /// 生成される名前付きパイプの設定:
    ///
    /// - 双方向 (duplex)
    /// - メッセージモード (読み取り・書き込みはメッセージ単位で行われる)
    /// - クライアント総数: 1つ
    /// - バッファーサイズ: 4MB (両方)
    /// - タイムアウト: 10秒
    pub(crate) fn new(name: &str) -> Self {
        let mut name_buf: Vec<u8> = Vec::with_capacity(260);
        name_buf.extend_from_slice(name.as_bytes());
        name_buf.push(0);

        let h = unsafe {
            CreateNamedPipeA(
                name_buf.as_mut_ptr() as *const i8,
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE,
                1,
                BUFFER_SIZE,
                BUFFER_SIZE,
                TIMEOUT,
                null_mut(),
            )
        };
        let h = h as RawHandle;

        // ハンドルを `OwnedHandle` に変換する。
        // その際、返り値が `INVALID_HANDLE_VALUE` ならエラーにする
        let h: OwnedHandle = {
            let h = unsafe { HandleOrInvalid::from_raw_handle(h) };
            OwnedHandle::try_from(h).unwrap_or_else(|_: InvalidHandleError| {
                let n = unsafe { GetLastError() };
                panic!("CreateNamedPipeA {n}")
            })
        };

        // ハンドルをRustの標準ライブラリにある `File` 構造体でラップする
        let file = File::from(h);
        Pipe { f: file }
    }

    /// パイプハンドルを複製する
    ///
    /// (同一のパイプを参照するオブジェクトをもう1つ作るということ。パイプがもう1つ作られるわけではない)
    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        let f = self.f.try_clone()?;
        Ok(Pipe { f })
    }

    /// パイプのクライアント側が開かれるまで待つ
    pub(crate) fn accept(&self) {
        let h = self.f.as_raw_handle() as HANDLE;

        if unsafe { ConnectNamedPipe(h, null_mut()) } == 0 {
            let n = unsafe { GetLastError() };
            if n == 535 {
                // ERROR_PIPE_CONNECTED
                eprintln!("[pipe] ERROR_PIPE_CONNECTED");
                return;
            }
            panic!("ConnectNamedPipe {n}")
        }
    }
}

impl Read for Pipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        Read::read(&mut self.f, buf)
    }
}

impl Write for Pipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Write::write(&mut self.f, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Write::flush(&mut self.f)
    }
}

// note: `File` がDrop時に `CloseHandle` するため、Pipeに対するDropトレイトの実装は必要ない

// ===============================================

#[cfg(skip)]
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, thread};

    #[test]
    fn t() {
        let barrier = std::sync::Barrier::new(2);

        thread::scope(|s| {
            s.spawn(|| {
                eprintln!("server: opening");
                let mut p = Pipe::new(r"\\.\pipe\hdg-pipe");

                eprintln!("server: barrier waiting");
                barrier.wait();

                eprintln!("server: accepting");
                p.accept();

                eprintln!("server: writing");
                write!(p, "ping\n").unwrap();

                eprintln!("server: reading");
                let mut buf = [0; 1024];
                let len = p.read(&mut buf).unwrap();
                let msg = &buf[0..len];
                eprintln!("server: recv {:?}", String::from_utf8_lossy(msg));
            });

            eprintln!("scope: waiting");
            barrier.wait();

            s.spawn(|| {
                eprintln!("client: opening");
                let mut q = fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(r"\\.\pipe\hdg-pipe")
                    .unwrap();

                eprintln!("client: reading");
                let mut buf = [0; 1024];
                let len = q.read(&mut buf).unwrap();
                let msg = &buf[0..len];
                eprintln!("client: recv {:?}", String::from_utf8_lossy(msg).as_ref());
                eprintln!("client: writing");
                write!(q, "pong\n").unwrap();
            });
        });
        // panic!("ok")
    }
}
