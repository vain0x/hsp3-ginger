# HSP3 フォージェリ

**WIP**: 開発中

**HSP3 フォージェリ** は HSP3 の静的解析ツール。

## プロジェクト

- [hf_core](./hf_core): パーサーや静的解析など
- [hf_lsp](./hf_lsp): LSP 実装
- [vscode-ext](./vscode-ext): VSCode 拡張機能

## 参考

- [rust-analyzer](https://github.com/rust-analyzer/rust-analyzer)
    - Rust の静的解析ツール。設計の参考にしている。
- [LSP Specification](https://microsoft.github.io/language-server-protocol/specifications/specification-current)
    - LSP の公式の仕様書
- [LSP学習記 #1](https://qiita.com/vain0x/items/d050fe7c8b342ed2004e#%E5%85%AC%E5%BC%8F%E3%81%AE%E3%82%B5%E3%83%B3%E3%83%97%E3%83%AB)
    - LSP に関する日本語の記事
- [vscode-extension-samples](https://github.com/microsoft/vscode-extension-samples)
    - VSCode 拡張機能のサンプル集。この中の lsp-sample を参考にしている。
