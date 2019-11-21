extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    // hspsdk.h を Rust に変換する。
    let bindings = bindgen::Builder::default()
        .header("./src/hspsdk/hspsdk.h")
        .generate()
        .expect("ERROR: hspsdk バインディングの生成に失敗しました");

    // 生成したバインディングをファイルとして出力する。
    // $OUT_DIR: Rust コンパイラのファイル出力先のディレクトリ。
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("hspsdk.rs"))
        .expect("ERROR: バインディングのファイル出力に失敗しました");
}
