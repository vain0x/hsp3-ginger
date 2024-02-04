# HSP3 アナライザー・ミニ

HSP3 の静的解析ツールです。Language Server Protocol (LSP) 準拠。

機能などは [vscode-ext/README.md](./vscode-ext/README.md) を参照。

- [ham-core](./ham-core): 解析機能および LSP サーバーの実装
    - [ham-lsp-server-dll](./ham-lsp-server-dll): DLL 版をビルドするためのプロジェクト
    - [ham-lsp-server-exe](./ham-lsp-server-exe): exe 版をビルドするためのプロジェクト
- [vscode-ext](./vscode-ext): VSCode 拡張機能

----

その他:

- [hsed3-ext](./hsed3-ext): hsed3 (標準のスクリプトエディタ) と連携するプロジェクト \[未完成\]

----

## 開発者用のドキュメント

この拡張機能の開発に関する資料は docs/dev/README.md を参照してください
