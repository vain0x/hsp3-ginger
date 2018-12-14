./build.ps1
if (!$?) { exit 1 }

cd vscode-ext
code --install-extension build.vsix
cd ..
