# hsp3-debug-adapter

Debug adapter protocol (DAP) の実装。

VSCode に HSP ランタイムの実行状況を送ったり、VSCode 側での操作 (ステップ実行や停止など) を HSP ランタイムに伝えたりする。

## ビルド

(ターミナルについては [dev.md](../dev.md) も参照。)

以下のコマンドをターミナルで実行する。

```sh
rustup toolchain install stable-i686-pc-windows-msvc
rustup toolchain install stable-x86_64-pc-windows-msvc

rustup target add i686-pc-windows-msvc
rustup target add x86_64-pc-windows-msvc
```

デバッグビルドは次のコマンドを使う:

```sh
cargo build --all-targets
```

リリースビルドは `--release` フラグをつける:

```sh
cargo build --release --all-targets
```
