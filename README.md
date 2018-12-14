# GINGER the HSP3 Debugger

*WIP: 開発中*

## 概要

*GINGER* は、VSCode で HSP3 のスクリプトのデバッグ実行を可能にするプロジェクト。

## 使いかた

[リリース](https://github.com/vain0x/hsp3-debug-ginger/releases) から VSIX をダウンロードして vscode にインストールする。VSCode のコマンドパレットに「VSIX からのインストール」がある。

- hsp (アーカイブ版) を新たにインストールする。 (**推奨**)
- chspcomp.exe, hsp3debug.dll, hsp3debug-ginger-adapter.dll を hsp のディレクトリに配置する。
    - [chspcomp.exe は公式サイトからダウンロードする](http://lldev.jp/others/freeware.html#chspcomp)
    - 後者2つは `C:\Users\<ユーザー名>\.vscode\extensions\vain0x.vscode-hsp3-debug-ginger-0.1.0\out\x86-sjis` にある。
- 新しいディレクトリを作って2つのファイルを作る。
    - main.hsp (中身は何でもいい)
    - [.vscode/launch.json](examples/.vscode/launch.json)
- VSCode でそのディレクトリを開いて、 main.hsp を開いた状態でデバッグ実行 (F5) を開始する。
- hsp のパスを聞かれるので、新規インストールした hsp のディレクトリを指定する。
- デバッグ実行が開始するはず。

## サポート状況

非 Windows 版、x64 版、utf8 版は未実装です。

## 構造

- `hsp3debug` (hsp3-debug-ginger)
    - C++。Windows 用の DLL を生成する。
    - HSPランタイムからアタッチされる。
    - `adapter` にアタッチして、イベントの仲介を行う。
- `adapter` (hsp3-debug-adapter)
    - Rust。Windows 用の DLL を生成する。
    - `hsp3debug` からアタッチされる。
    - TCP クライアントを実行して、 `middle-adapter` 経由で VSCode と通信する。
    - HSP ランタイムの内部状態を解析して、デバッガーアダプタープロトコルにのっとったメッセージを生成する。
- `middle-adapter` (hsp3-debug-ginger-middle)
    - Rust。コンソールアプリ。
    - TCP サーバーを起動して、 VSCode と `adapter` の通信を中継する。
- `vscode-ext` (vscode-hsp3-debug-ginger)
    - TypeScript
    - VSCode のデバッガー拡張機能から呼び出される。

## 開発

GINGER の開発者のためのメモ: [dev.md](./dev.md)

## 参考

- [OpenHSP](http://dev.onionsoft.net/trac)
- [knowbug](https://github.com/vain0x/knowbug)
