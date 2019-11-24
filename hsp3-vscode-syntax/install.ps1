#!/bin/pwsh

./setup
if (!$?) {
    exit 1
}

npm run vsce:package
code --install-extension 'hsp3-vscode-syntax.vsix'
