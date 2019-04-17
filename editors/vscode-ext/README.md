# HSP3 Ginger for VSCode

## 開発環境

- [Node.js](https://nodejs.org)
- [Yarn](https://yarnpkg.com)

## インストール

```
cp ../../LICENSE LICENSE
cp ../../lib/language-hsp3/grammars/hsp3.json syntaxes/hsp3.json

yarn
yarn vsce package --yarn --out ginger.vsix
code --install-extension ginger.vsix
```

```
code --uninstall-extension "vain0x.hsp3-ginger"
```
