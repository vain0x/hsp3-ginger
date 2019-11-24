#!/bin/pwsh

npm --version
if (!$?) {
    echo 'Node.js をインストールしてください。'
    exit 1
}

npm intall

cp '../LICENSE' 'LICENSE'
cp '../language-hsp3/grammars/hsp3.json' 'syntaxes/hsp3.json'
