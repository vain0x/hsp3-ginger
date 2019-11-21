# すべての構成でデバッガーをビルドします。

if (!$(which MSBuild.exe)) {
    write-error 'MSBuild.exe にパスを通してください。'
    exit 1
}

function build($config) {
    $sln = './hsp3-debug-spider.sln'

    MSBuild.exe $sln '-t:Build' $config
}

build '-p:Configuration=Debug;Platform=x86'
build '-p:Configuration=DebugUtf8;Platform=x86'
build '-p:Configuration=Debug;Platform=x64'
build '-p:Configuration=Release;Platform=x86'
build '-p:Configuration=ReleaseUtf8;Platform=x86'
build '-p:Configuration=Release;Platform=x64'
