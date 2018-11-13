extern crate libc;
extern crate ws;

#[cfg(feature = "winuser")]
extern crate winapi;

mod connection;
mod helpers;
mod hspsdk;
mod logger;

#[cfg(feature = "winuser")]
mod win;
