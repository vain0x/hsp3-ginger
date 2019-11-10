# hsp3-debug-spider

## 開発環境

- Windows 10
- Visual Studio 2019 Community
- Rust tools

### 開発: Rust tools のインストール

<https://rustlang.org>

```rust
rustup toolchain install stable-i686-pc-windows-msvc
rustup target add i686-pc-windows-msvc
```

### 開発: ビルドターゲット

- コンフィグレーション
    - Debug/Release
        - Debug はデバッガー自体をデバッグするためのモード。
        - Release はデバッガーを配布するためのモード。
        - shift_jis ランタイム用
    - DebugUtf8/ReleseUtf8
        - UTF-8 ランタイム用
- プラットフォーム
    - x86 (Win32): 32ビット版
    - x64: 64ビット版 (hsp3debug_64.dll)

### 開発: ビルドスクリプト

scripts/README.md を参照。

```rust
cargo build --target=i686-pc-windows-msvc
```

## ライセンス

HSPSDK のライセンスは licenses/openhsp を参照してください。

それ以外はパブリックドメイン (著作権なし) として扱います。
