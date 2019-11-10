# hsp3-debug-empty

HSP3 の「何もしない」デバッガーです。

新たにデバッガーを作るときの土台として利用できます。

## 開発環境

- Windows 10
- Visual Studio 2019 Community

### 開発: ビルドターゲット

- コンフィグレーション
    - Debug/Release
        - Debug はデバッガー自体をデバッグするためのモード。
        - Release はデバッガーを配布するためのモード。
        - shift_jis ランタイム用
    - DebugUtf8/ReleseUtf8
        - UTF-8 ランタイム用
- プラットフォーム
    - x86 (Win32): 32ビット版
    - x64: 64ビット版 (hsp3debug_64.dll)

### 開発: プロジェクト設定

プロジェクト設定の既定値との差異は以下を参照してください。

- <https://github.com/vain0x/hsp3-ginger/commit/02cd818>
    chore: 出力ディレクトリ・中間ディレクトリを target に変更
- <https://github.com/vain0x/hsp3-ginger/commit/6c77cc7>
    chore: ターゲット名 (DLLファイル名) を設定
- <https://github.com/vain0x/hsp3-ginger/commit/a1814fe>
    chore: UTF-8 版で HSP3_UTF8 マクロを定義
- <https://github.com/vain0x/hsp3-ginger/commit/693541c>
    chore: ビルド構成 DebugUtf8/ReleaseUtf8 を追加
- <https://github.com/vain0x/hsp3-ginger/commit/383b8ff>
    chore: 言語標準を C++17 に変更
- <https://github.com/vain0x/hsp3-ginger/commit/1a971ae>
    chore: コンパイラオプション /utf-8 を有効化
- <https://github.com/vain0x/hsp3-ginger/commit/ce27db2>
    chore: _WINDOWS マクロを定義

## ライセンス

HSPSDK のライセンスは licenses/openhsp を参照してください。

それ以外はパブリックドメイン (著作権なし) として扱います。
