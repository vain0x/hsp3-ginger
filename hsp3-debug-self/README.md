# hsp3-debug-self

クライアントの実装も HSP3 で書いているデバッガー。

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

### 開発: ビルドスクリプト

scripts/README.md を参照。

```pwsh
new-item -itemType symbolicLink -path hds-client -value C:\path\to\hsp3-ginger\projects\hsp3-debug-self\hds-client

copy-item hds-client/hds_client_proxy.exe C:\path\to\hsp3-ginger\projects\hsp3-debug-self\hds-client\hds_client_proxy.exe
```

## ライセンス

HSPSDK のライセンスは licenses/openhsp を参照してください。
