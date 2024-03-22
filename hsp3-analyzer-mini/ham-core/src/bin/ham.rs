// HAM CLI: HAMの機能をコマンドラインアプリとして提供する (予定)

// 開発環境での起動方法:
//
// ```sh
// HSP3_ROOT=C:/.../hsp3x cargo run --bin ham
// ```

use ham_core::commands;

fn main() {
    commands::profile_parse();
}
