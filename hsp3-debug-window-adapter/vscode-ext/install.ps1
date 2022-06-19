#!/bin/pwsh

npm --version
if (!$?) {
    echo 'Node.js をインストールしてください。'
    exit 1
}

npm ci --ignore-scripts
npm run build
npm run vsce:package
code --install-extension 'hsp3-debug-window-adapter.vsix'
