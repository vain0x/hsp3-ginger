//! HSPSDK bindings for Rust.
//!
//! ビルド中に、HSPCTX などの構造体をC言語と互換性のあるフォーマットで定義する Rust のコードが生成される。
//! 生成物は target/debug/build/hsp3debug-*/out/hspsdk.rs にある。
//! 以下の include! マクロにより、生成されたコードがこのファイル (hspsdk モジュール) に含まれているかのように扱われる。
//! build.rs と bindgen のドキュメントを参照。

#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(warnings)]

pub(crate) type DebugMode = i32;

include!(concat!(env!("OUT_DIR"), "/hspsdk.rs"));
