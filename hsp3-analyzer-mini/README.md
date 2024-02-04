# HSP3 アナライザー・ミニ

HSP3 の静的解析ツールです。Language Server Protocol (LSP) 準拠。

機能などは [vscode-ext/README.md](./vscode-ext/README.md) を参照。

- [ham-core](./ham-core): 解析機能および LSP サーバーの実装
    - [ham-lsp-server-dll](./ham-lsp-server-dll): DLL 版をビルドするためのプロジェクト
    - [ham-lsp-server-exe](./ham-lsp-server-exe): exe 版をビルドするためのプロジェクト
- [vscode-ext](./vscode-ext): VSCode 拡張機能

その他:

- [hsed3-ext](./hsed3-ext): hsed3 (標準のスクリプトエディタ) と連携するプロジェクト \[未完成\]

----

## 開発環境

この拡張機能を開発する環境の構築手順は以下の通りです。

以下のツールをインストールしてください。

- [Node.js](https://nodejs.org) (>= 17)
- [Rust](https://rustlang.org)

はじめに、パッケージのインストールなどが必要です。次のスクリプトを使ってください。

```sh
./setup
```

以上で環境構築は完了です。他の操作には、以下のスクリプトを使用します。

- `./build`: ビルド
- `./install`: VSCode に拡張機能をインストールする
- `./uninstall`: VSCode から拡張機能をアンインストールする

### デバッグ実行

VSCodeの「フォルダーを開く」でこのファイルがあるフォルダーを開きます (このウィンドウをW1とします)。
デバッグ実行 (F5) を開始すると、VSCodeの新しいウィンドウ (W2とします) が開かれます。
このウィンドウでは、開発中の拡張機能がインストールされています。
適当な `.hsp` ファイルを開いて動作確認できます

デバッグ実行の設定は `.vscode/launch.json` に書かれています。
デバッグ実行に関して、さらに詳しく知るには、「VSCodeの拡張機能のデバッグ」などで検索するとよいでしょう

ログの出力先は次のとおりです:

- Rust:
    `debug!()` などのマクロでログ出力できます。
    そのウィンドウで開いているフォルダーに `ham-lsp.log` というファイルが生成されて、そこに書き込まれます
- extension:
    `console` を使ってログ出力できます。デバッグ実行中のウィンドウ(W1)の「デバッグコンソール」パネルに表示されます
- vscode-languageclient:
    VSCodeの設定で `hsp3-analyzer-mini.trace.server` の値を変更して、LSPメッセージをログ出力してもらうこともできます。
    これらのメッセージはデバッグ対象のウィンドウ(W2)の「出力」パネルで「HSP3アナライザー・ミニ」を選択すると閲覧できます

### テスト

テストは `cargo test` で実行します。(テストケースの網羅性は低いです)

一部のテストは hsp3 のインストールディレクトリに含まれているモジュールやサンプルコードを参照します。
そのため `vendor/hsp3` に hsp3 (zip版) をインストールしておく必要があります。

## 関連リンク

- [LSP学習記 #1](https://qiita.com/vain0x/items/d050fe7c8b342ed2004e)
    - LSP の学んだことをまとめた連載記事です。
