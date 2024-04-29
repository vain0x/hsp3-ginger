# HSP3 アナライザー・ミニ

HSP3 の静的解析ツールです。Language Server Protocol (LSP) 準拠。

機能などは [vscode-ext/README.md](./vscode-ext/README.md) を参照。

- [ham-core](./ham-core): 解析機能および LSP サーバーの実装
    - [bin/ham](./ham-core/src/bin/ham.rs): 実行ファイルのエントリーポイント
- [vscode-ext](./vscode-ext): VSCode 拡張機能

----

その他:

- [ham-sdk](./ham-sdk): DLL版をビルドするためのプロジェクト (未完成)
- [hsed3-ext](./hsed3-ext): hsed3 (標準のスクリプトエディタ) と連携するプロジェクト \[未完成\]

----

## 開発者用のドキュメント

この拡張機能の開発に関する資料は ./docs/dev にあります
