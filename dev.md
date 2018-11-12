# 開発ノート

## 開発環境

- OS: Windows 10
- Visual Studio 2017 Community をインストールする。
    - 「C++ のデスクトップ開発」の項目にチェックがついていることを確認。
    - hsp3debug の開発に使う。デバッガーのデバッグにも使える。
    - これをインストールするときについてくる Visual Sutiod 2017 Build Tools が Rust でも必要になる。
- Rust をインストールする。
    - [Rust](https://www.rust-lang.org)
    - デバッガーの本体の開発に使う。
- Node.js をインストールする。
    - [Node.js](https://nodejs.org)
    - インストールすると node と npm 入る。
    - VSCode 向けの拡張機能の開発に使う。

その他:

- \[省略可\] VSCode
    - テキストエディタ。Rust や Typescript での開発に使う。
    - 入れておくとよい拡張機能:
        - Rust (rls)
        - EditorConfig

## Appendix A. ターミナル (PowerShell)

開発環境の構築時やコンパイル時などに PowerShell をよく使う。

### ターミナル: PowerShell の開きかた

ディレクトリを Shift キーを押しながら右クリックして、「ここで PowerShell を開く」を使う。

または、VSCode の統合ターミナルを使う。

### ターミナル: 備考

- 古い PowerShell には文字コードの問題があるので、 pwsh 6 (執筆時点ではpreview版) を入れたほうがいい。 \[省略可\]
- VSCode の統合ターミナルで pwsh.exe を使う。 \[省略可\]

## Appendix B. シンボリックリンク

シンボリックリンクを使うと便利である。例えば次の手順によって、ビルド時に生成された hsp3debug.dll が HSP ディレクトリに存在するかのようにみせかけることができる。

- PowerShell を管理者権限で実行する。
    - Windows のアイコンを右クリックして「Windows PowerShell (管理者)」を選ぶ。
- cd コマンドで作業ディレクトリを HSP のディレクトリに移動する。
- new-item コマンドでシンボリックリンクを生成する。

例:

```powershell
cd "C:/Program Files/hsp35"

new-item -itemType symbolicLink -path hsp3debug.dll -value "C:/repo/hsp3-debug-ginger/hsp3debug/Debug/hsp3-debug-ginger.dll"
```

ファイルパスは読者の環境に合わせて修正する必要がある。上記では HSP が `C:/Program Files/hsp35` にインストールされていて、GINGER のリポジトリが `C:/repo` 直下にクローンされているとしている。

この操作 (`new-item` コマンド) により、シンボリックリンク `C:/Program Files/hsp35/hsp3debug.dll` が生成される。これは `略/hsp3-debug-ginger.dll` にあるファイルと同一のものとみなされる。

- シンボリックリンクは普通のファイルと同様に削除できる。
