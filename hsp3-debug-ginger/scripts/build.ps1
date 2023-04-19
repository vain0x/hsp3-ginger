cargo --version
if (!$?) { exit 1 }

npm --version
if (!$?) { exit 1 }

vsce --version
if (!$?) {
    npm instal -g vsce
    if (!$?) { exit 1 }
}

cd hsp3debug
msbuild /p:Configuration=Release /p:Platform=x86
cd ..
cp ./hsp3debug/Release/hsp3-debug-ginger.dll ./vscode-ext/out/x86-sjis/hsp3debug.dll

cd middle-adapter
cargo build --release
cd ..
cp ./target/middle-adapter/release/middle-adapter.exe ./vscode-ext/out/middle-adapter.exe

cd adapter
cargo build --release --target i686-pc-windows-msvc
cd ..
cp ./target/adapter/i686-pc-windows-msvc/release/hsp3_debug_adapter.dll ./vscode-ext/out/x86-sjis/hsp3debug-ginger-adapter.dll

cd vscode-ext
npm run compile
if (!$?) { exit 1 }
vsce package -o build.vsix
if (!$?) { exit 1 }
cd ..
