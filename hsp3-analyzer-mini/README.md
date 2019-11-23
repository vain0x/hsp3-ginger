# HSP3 Analyzer Mini

HSP3 用の Language Server Protocol (LSP) サーバーです。

**実装が雑** なので誤った情報を表示することがあります。ご了承ください。

## インストール

- [Visual Studio Code](https://code.visualstudio.com) をインストールしてください。
- 起動したら `Ctrl+Shift+P` で「コマンドパレット」を開き、install extension と入力して "Extensions: Install Extensions" のメニュー項目を押してください。
- 表示される検索欄を使って以下の拡張機能を検索し、インストールしてください。
    - `language-hsp3` で検索して install してください
    - `hsp3-analyzer-mini` で検索して install してください
    - (ちなみに `japanese language pack` を install すると VSCode が日本語化されます)
- `Ctrl+Shift+P` でコマンドパレットを開き、open user settings json と入力して "Open Users Settings (JSON)" を選び、開かれたファイルの内容を以下のように変更してください。
    - HSP のディレクトリのパスは、HSPで `mes dir_exe` を実行すると確認できます。

```json
{
    "hsp3-analyzer-mini.hsp3-root": "<HSPのディレクトリのパス>",
    "[hsp3]": {
        "files.autoGuessEncoding": true
    }
}
```

例:

```json
{
    "hsp3-analyzer-mini.hsp3-root": "C:\\Program Files (x86)\\hsp351",
    "[hsp3]": {
        "files.autoGuessEncoding": true
    }
}
```

その後、HSP のファイル (.hsp ファイル) を開くと動作するはずです。

## 機能

- ホバー
    - 変数や命令などにカーソルをのせると関連情報が表示されます。
    - hsphelp にヘルプファイルがある場合は、その内容の一部 (パラメータ情報と説明) を表示します。
    - あるいは、定義箇所 (`#deffunc` など) の上にあるコメントを表示します。
- 入力補完
    - Ctrl+Space で入力補完 (オートコンプリート) の一覧が表示されます。
- 定義・使用箇所の参照
    - 変数や命令を選択して右クリックメニューから「定義へ移動」(F12)をすると定義箇所に移動します。(Alt+← で戻る。)
    - 「すべての参照を表示」(Shift+F12)で定義・使用箇所の一覧を表示します。

## 定義・使用箇所

変数や命令など識別子が定義されている場所を定義箇所 (def-site)、使用されている場所を使用箇所 (use-site) と呼びます。

例えば以下のスクリプトでは、最初の行でラベル `*l_main` が「定義」されていて、ここがラベルの定義箇所です。一方、`goto *l_main` ではラベルが「使用」されていて、ここがラベルの使用箇所です。

```hsp
*l_main
    await 17
    goto *l_main
```

この LSP は変数の定義・使用箇所を正確に解析できません。代わりに、代入や `dim` 命令などを見つけたら変数が定義されたとみなすことにしています。

```hsp
    // dim 命令の最初の引数に指定されたので、変数 x が「定義」されたとみなす
    dim x

    // = の左辺にあるので、変数 y が「定義」されたとみなす
    y = 1
```

繰り返しになりますが、この LSP による定義・使用箇所の検査は不正確なので、過信しないでください。

## その他

- エディターで開いているファイルと同じディレクトリ (またはその下にディレクトリ) にある `.hsp` ファイルも自動的に解析されます。
- 動作がおかしくなったら再起動してみてください。(`Ctrl+Shift+P` → "Reload Windows")

## 未対応

- `#include`
- `#func`, `#cfunc`, `#cmd`, `#comfunc`
- `#undef`
- 変数名についた `@`
- `.as` ファイルの解析
- common ディレクトリにあるファイルの解析
- カレントディレクトリにあるヘルプソースファイルの解析

## 参照

- [language-hsp3](https://github.com/honobonosun/vscode-language-hsp3)
    - リンク先の手順に従うと VSCode 上で HSP のデバッグ実行などが可能になるようです。
- [LSP学習記 #1](https://qiita.com/vain0x/items/d050fe7c8b342ed2004e)
    - LSP の学んだことをまとめた連載記事です。