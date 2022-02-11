[CmdletBinding()]
param(
    [Parameter(Mandatory=$false)]
    [string] $Version,
    [switch] $Release,
    [string] $OutDir = '.\.release',
    [switch] $Installer,
    [ValidateSet("x64","x86")]
    [string[]] $Architecture = @("x64","x86")
)

# リリースビルドの場合vcのライブラリをスタティックリンクする
if ($Release) {
    $env:RUSTFLAGS='-C target-feature=+crt-static'
} else {
    $env:RUSTFLAGS=''
}

# ビルド
if (! $Installer -or ($Release -and $Installer)) {
    if ("x64" -in $Architecture) {
        # build x64 exe
        $cmd = 'cargo build {0}' -f $(if ($Release) {'--release'})
        Invoke-Expression -Command $cmd
    }
    if ("x86" -in $Architecture) {
        # build x86 exe
        $cmd = 'cargo build --target=i686-pc-windows-msvc {0}' -f $(if ($Release) {'--release'})
        Invoke-Expression -Command $cmd
    }
}

# 出力先フォルダを作成
if (! (Test-Path($OutDir))) {
    mkdir $OutDir | Out-Null
}

function Get-BinaryVersion {
    [OutputType()]
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        $BinPath
    )
    process {
        if (! (Test-Path $BinPath)) {
            Write-Error "$($BinPath) が見つかりません"
            break
        }
        if (('{0} --version' -f $BinPath | Invoke-Expression) -match '\d+\.\d+\.\d+') {
            $Version = $Matches[0]
        } else {
            Write-Error "uwscrのバージョンが不明"
            break
        }
        $Version
    }
}
function Out-UWSCR {
    [OutputType()]
    [CmdletBinding(DefaultParameterSetName="both")]
    param(
        [Parameter(Mandatory)]
        $BinPath,
        [Parameter(Mandatory,ParameterSetName="x64")]
        [switch] $x64,
        [Parameter(Mandatory,ParameterSetName="x86")]
        [switch] $x86
    )
    process {
        if (! $Version) {
            $Version = Get-BinaryVersion -BinPath $BinPath
        }
        $Arch = $x64 ? "x64": "x86"
        $verpath = Join-Path -Path $OutDir -ChildPath $Version
        $ArchDir = Join-Path -Path $verpath -ChildPath $Arch
        if (! (Test-Path $verpath)) {
            mkdir $verpath | ForEach-Object {
                mkdir $ArchDir | Out-Null
            }
        }
        $BinPath | Copy-Item -Destination $ArchDir
        $ZipPath = Join-Path -Path $verpath -ChildPath "UWSCR$Arch.zip"
        Get-ChildItem $BinPath | Compress-Archive -DestinationPath $ZipPath -Force
        Get-Item $ZipPath
    }
}

if ($Release) {
    $env:RUSTFLAGS=''

    if ("x64" -in $Architecture) {
        Out-UWSCR -BinPath '.\target\release\uwscr.exe' -x64
    }
    if ("x86" -in $Architecture) {
        Out-UWSCR -BinPath '.\target\i686-pc-windows-msvc\release\uwscr.exe' -x86
    }

}

# msi installer
if ($Installer) {
    # requires wix toolset
    if (! (Get-Command candle,light -ea SilentlyContinue | Where-Object Source -Match 'WiX Toolset')) {
        Write-Warning "WiX Toolsets not found"
        break;
    }

    # x64 for default
    if ("x64" -in $Architecture) {
        $exe64 = '.\target\release\uwscr.exe'
        $v = Get-BinaryVersion -BinPath $exe64
        if ($v) {
            if (! $Version) {$Version = $v}
            # cargo wix --nocapture
            candle -dProfile=release -dVersion="${Version}" -dPlatform=x64 -ext WixUtilExtension -o target/wix/x64.wixobj wix/x64.wxs -nologo | Out-Null
            $msipath = ".release/${Version}/uwscr-${Version}-x64.msi"
            light -spdb -ext WixUIExtension -ext WixUtilExtension -cultures:ja-JP -out $msipath target/wix/x64.wixobj -nologo | Out-Null
            Get-Item $msipath
        }
    }
    # x86
    if ("x86" -in $Architecture) {
        $exe86 = '.\target\i686-pc-windows-msvc\release\uwscr.exe'
        $v = Get-BinaryVersion -BinPath $exe86
        if ($v) {
            if (! $Version) {$Version = $v}
            # cargo wix --compiler-arg "-dProfile=i686-pc-windows-msvc\release -dPlatform=x86" --nocapture
            candle -dProfile=i686-pc-windows-msvc\release -dVersion="${Version}" -dPlatform=x86 -ext WixUtilExtension -o target/wix/x86.wixobj wix/x86.wxs -nologo | Out-Null
            $msipath = ".release/${Version}/uwscr-${Version}-x86.msi"
            light -spdb -ext WixUIExtension -ext WixUtilExtension -cultures:ja-JP -out $msipath target/wix/x86.wixobj -nologo | Out-Null
            Get-Item $msipath
        }
    }
}
