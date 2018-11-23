# GINGER the HSP3 Debugger

*WIP: 開発中*

## 概要

*GINGER* は、VSCode で HSP3 のスクリプトのデバッグ実行を可能にするプロジェクト。

## 使いかた

まだ使えません。

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
