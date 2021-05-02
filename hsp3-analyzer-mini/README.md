# HSP3 アナライザー・ミニ

HSP3 の実用最小限な静的解析ツールです。Language Server Protocol (LSP) 準拠。

機能などは [vscode-ext/README.md](./vscode-ext/README.md) を参照。

- [ham-core](./ham-core): 解析機能および LSP サーバーの実装
    - [ham-lsp-server-dll](./ham-lsp-server-dll): DLL 版をビルドするためのプロジェクト
    - [ham-lsp-server-exe](./ham-lsp-server-exe): exe 版をビルドするためのプロジェクト
- [vscode-ext](./vscode-ext): VSCode 拡張機能

----

## 開発環境

この拡張機能を開発する環境の構築手順は以下の通りです。

以下のツールをインストールしてください。

- [Node.js](https://nodejs.org)
- [Rust](https://rustlang.org)

はじめに、パッケージのインストールなどが必要です。次のスクリプトを使ってください。

```sh
./setup
```

以上で環境構築は完了です。他の操作には、以下のスクリプトを使用します。

- `./build`: ビルド
- `./install`: VSCode に拡張機能をインストールする
- `./uninstall`: VSCode から拡張機能をアンインストールする

### テスト

テストは `cargo test` で実行します。

一部のテストは hsp3 のインストールディレクトリに含まれているモジュールやサンプルコードを参照します。
そのため `vendor/hsp3` に hsp3 (zip版) をインストールしておく必要があります。

## 関連リンク

- [LSP学習記 #1](https://qiita.com/vain0x/items/d050fe7c8b342ed2004e)
    - LSP の学んだことをまとめた連載記事です。
