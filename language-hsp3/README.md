# language-hsp3
このパッケージは、HSP3言語の文法をサポートするための機能を含んでいます。  
HSP3 language support for atom

<img src="https://raw.githubusercontent.com/honobonosun/language-hsp3/master/ss.png" alt="ss" title="ss">

## パッケージの導入方法
1. apmを使用する場合、以下のコマンドを実行してください。
   ```install.bat
   apm install language-hsp3
   ```
   もしくは、設定画面のインストールタブで「language-hsp3」を検索して、インストールします。

2. HSP3ソースファイルをコンパイルするための **コンソールコンパイラ** を別途、インストールする必要があります。
    - language-hsp3 は [hspc](http://dev.onionsoft.net/seed/info.ax?id=1392) を既定ツールとして、対応しています。
    - その他のツールは、完全な互換性がない事にご承知上、ご利用くださいますようお願い申し上げます。

3. コンソールコンパイラをインストールしたパスを設定してください。
    - HSP3.5を既定のフォルダ（ディレクトリ）にインストールして、hspcをreadmeに従って、HSP3.5にインストールした場合、設定は不要です。
    - 上記に当てはまらない場合
      1. コンソールコンパイラのパスを「Compiler Settings」の「Compiler path」に設定してください。
      2. 「Compile Command Settings」の各コマンド引数に、<kbd>F5</kbd>と<kbd>Ctrl-F9</kbd>が、該当するコマンド引数を設定してください。

4. 以下のコードをhello.hspとしてファイル保存後、<kbd>F5</kbd>を押してSuccess通知が表示されれば完了です。
   ```hello.hsp
   end
   ```

## 機能
HSP3開発環境をatomで提供するために、以下の機能をサポートします。

- HSP3言語の色分け
- HDL形式ドキュメントのサポート
    - ドキュメント付けされたヘッダファイルのサポート
- 外部コンソールツールを使用したコマンド実行
    1. コンパイル＋実行(<kbd>F5</kbd>)
    2. 自動実行ファイル作成(<kbd>Ctrl-F9</kbd>)
- UTF-8の対応

### HSP3言語の色分け
HSPファイル(\*\*.hsp, \*\*.as)に対して、色分け機能を提供します。

mesなどの標準機能のキーワードはもちろん、#defineの特殊文字なども色分けされます。
また、一部コンパイルできないコードを強調表示します。

### HDL形式ドキュメントのサポート
HSP Document Library で使用する、HSファイル（バージョン2.0）の色分けとスニペットに対応しています。

%sampleフィールドタグでは、HSP3言語で色分けされます。
%instフィールドタグで**html{～}html**を使用したブロックは、HTMLで色分けされます。

#### ドキュメント付けされたヘッダファイルのサポート
HSPファイル内の複数行コメントでも、HSファイルと同じ色分け機能を提供します。

### 実行可能なコマンド
エディタの言語モードが、HSP3の場合、以下のショートカットキーが有効になります。

|key map|command           |実行内容           |
|------:|:-----------------|------------------|
|F5     |language-hsp3:run |コンパイル＋実行    |
|Ctrl-F9|language-hsp3:make|自動実行ファイル作成|

<kbd>F5</kbd>で「コンパイル＋実行」コマンドを実行します。
<kbd>Ctrl-F9</kbd>で「自動実行ファイル作成」コマンドを実行します。

その他に、メニューバーもしくはエディタのコンテクストメニューからコマンドを実行できます。

コマンド呼び出しで使用されるコンソールコンパイラは、パッケージ設定画面から編集することができます。
コンソールコンパイラに対するパラメータは、コマンド毎に設定できます。
`,`で区切ることで、複数のパラメータを渡すことができます。

パラメータには、`%`で囲まれた置き換え文字が使用できます。

|string    |置き換え文字                                          |
|---------:|:----------------------------------------------------|
|%FILEPATH%|現在開いているエディタのファイルパス                    |
|%PROJECT% |現在開いているエディタのプロジェクトルートディレクトリパス|

上記の文字は、コンソールコンパイラを呼び出す際に置き換わります。

パスに空白文字がある場合、パッケージ設定の *Extension Option Settings* の *Delete quotation character* オプションの有無で`"`が添削されます。

### UTF-8の対応
hspcを導入した環境で説明します。

コードページがshift_jisのファイルをhsp3utfランタイムで動作させるには、ソースコードに`#include "hsp3utf.as"`を挿入してください。パラメータには、*-u* オプションを追加指定してください。

コードページがutf-8のファイルをhsp3utfランタイムで動作されるには、ソースコードに`#runtime "hsp3utf"`を挿入してください。パラメータには、*-i* もしくは *-a* オプションを追加指定してください。*-a* オプションでファイルのコードページが正しく推測できていない場合、コード先頭に`; ユニコードだよ`等の文字化け対策のコメントを挿入してください。

shift_jis、utf-8を混合して編集する場合は、*-a* オプションの追加指定を推奨します。

hspcは、コードページが統一されているのを前提に処理します。コードページが異なるファイルをincludeした場合、構文エラーまたは文字化けを引き起こす可能性に留意してください。

### flex-tool-bar 設定例

以下のコードは、エディタの言語モードがHSP3の場合のみ "実行" コマンドを行うボタンを表示する設定です。

```cson
{
  type: "button"
  tooltip: "HSP3 Compile and Run"
  icon: "triangle-right"
  callback: "language-hsp3:run"
  show:
    grammar: "HSP3"
  style:
    color: "green"
}
```

## 使用したコードとライセンス表記
### language-hsp3
MIT License  
Copyright (c) 2017-2018 Honobono

### iconv-lite
MIT License  
<https://www.npmjs.com/package/iconv-lite>

Copyright (c) 2011 Alexander Shtuchkin

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
