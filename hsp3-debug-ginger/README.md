# GINGER the HSP3 Debugger

*WIP: 開発中。動作しません*

## 概要

*GINGER* は、VSCode で HSP3 のスクリプトのデバッグ実行を可能にするプロジェクト。

## 使いかた

### 使いかた: インストール手順

アルファ版をリリースしているので、そのインストール手順について簡単に解説する。

- [最新のリリース](https://github.com/vain0x/hsp3-debug-ginger/releases/latest) から build.vsix をダウンロードして VSCode にインストールする。
    - VSCode のコマンドパレットに「VSIX からのインストール」がある。
- hsp (アーカイブ版) を新たにインストールする。 (**推奨**)
- chspcomp.exe, hsp3debug.dll, hsp3debug-ginger-adapter.dll を hsp のディレクトリに配置する。
    - [chspcomp.exe は公式サイトからダウンロードする](http://lldev.jp/others/freeware.html#chspcomp)
    - 後者2つは `C:\Users\<ユーザー名>\.vscode\extensions\vain0x.vscode-hsp3-debug-ginger-0.1.0\out\x86-sjis` にある。
- 新しいディレクトリを作って2つのファイルを作る。
    - main.hsp (中身は何でもいい)
    - [.vscode/launch.json](examples/.vscode/launch.json)
- VSCode でそのディレクトリを開いて、 main.hsp を開いた状態でデバッグ実行 (F5) を開始する。
- ディレクトリの選択を求められるので、hsp のディレクトリを指定する。
    - 初回のみ。指定したパスはユーザーの設定ファイルに記録される。
- デバッグ実行が開始するはず。

### 使いかた: 実装済みの機能

- 実行位置の追跡
- ステップ実行
- str/double/int 型のグローバル変数の値の表示
    - 1次元配列の場合は各要素の表示

### 使いかた: 既知の不具合

- ステップオーバーとステップアウトがステップインと同じ挙動をする。
- ステップインが遅い。また、変数ビューの選択状態が維持されない。
    - ステップインをダブルクリックすると高速に処理されるケースがある。
- デバッグ終了時、プログラムが停止しないことがある。
- 非 Windows 版HSP、 64ビット版HSP、utf8版HSPには未対応。

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
