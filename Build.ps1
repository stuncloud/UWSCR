[CmdletBinding()]
param(
    [Parameter(Mandatory=$false)]
    [string] $Version,
    [switch] $Release,
    [string] $OutDir = '.\.release',
    [switch] $Installer,
    [switch] $Schema,
    [ValidateSet("x64","x86")]
    [string[]] $Architecture = @("x64","x86"),
    # opencv x64版installビルド先のパス
    [string] $OpenCV64Install,
    # opencv x86版installビルド先のパス
    [string] $OpenCV86Install,
    [switch] $Document,
    [switch] $Clean
)

trap {
    break
}

# リリースビルドの場合vcのライブラリをスタティックリンクする
if ($Release) {
    $env:RUSTFLAGS='-C target-feature=+crt-static'
} else {
    $env:RUSTFLAGS=''
}

# ビルド
if ((! $Installer) -or ($Release -and $Installer)) {
    if ("x64" -in $Architecture) {
        # build x64 exe
        if ($OpenCV64Install -ne $null) {
            $env:OPENCV_INCLUDE_PATHS = Join-Path $OpenCV64Install 'include'
            $env:OPENCV_LINK_PATHS = Join-Path $OpenCV64Install 'x64\vc16\staticlib'
            $libs = (Join-Path $env:OPENCV_LINK_PATHS '*.lib' | Get-ChildItem).BaseName | ForEach-Object {
                    $_.StartsWith('lib') ? "lib$($_)" : $_
            }
            $env:OPENCV_LINK_LIBS = $libs -join ','
            Write-Verbose $env:OPENCV_INCLUDE_PATHS
            Write-Verbose $env:OPENCV_LINK_PATHS
            Write-Verbose $env:OPENCV_LINK_LIBS
            if ($Release) {
                cargo build --features chkimg --release
            } else {
                cargo build --features chkimg
            }
        } else {
            if ($Release) {
                cargo build --release
            } else {
                cargo build
            }
        }
        if ($LASTEXITCODE -ne 0) {
            break
        }
    }
    if ("x86" -in $Architecture) {
        # build x86 exe
        if ($OpenCV86Install -ne $null) {
            $env:OPENCV_INCLUDE_PATHS = Join-Path $OpenCV86Install 'include'
            $env:OPENCV_LINK_PATHS = Join-Path $OpenCV86Install 'x86\vc16\staticlib'
            $libs = (Join-Path $env:OPENCV_LINK_PATHS '*.lib' | Get-ChildItem).BaseName | ForEach-Object {
                    $_.StartsWith('lib') ? "lib$($_)" : $_
            }
            $env:OPENCV_LINK_LIBS = $libs -join ','
            Write-Verbose $env:OPENCV_INCLUDE_PATHS
            Write-Verbose $env:OPENCV_LINK_PATHS
            Write-Verbose $env:OPENCV_LINK_LIBS
            if ($Release) {
                cargo build --target=i686-pc-windows-msvc --features chkimg --release
            } else {
                cargo build --target=i686-pc-windows-msvc --features chkimg
            }
        } else {
            if ($Release) {
                cargo build --target=i686-pc-windows-msvc --release
            } else {
                cargo build --target=i686-pc-windows-msvc
            }
        }
        if ($LASTEXITCODE -ne 0) {
            break
        }
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
        $bin = Get-Item -Path $BinPath
        $Version = $bin.VersionInfo.FileVersion
        Write-Verbose $Version
        if ($Version.Length -eq 0) {
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
        if ($Checkimg) {$Arch += "_chkimg"}
        $verpath = Join-Path -Path $OutDir -ChildPath $Version
        $ArchDir = Join-Path -Path $verpath -ChildPath $Arch
        mkdir $verpath -Force | Out-Null
        mkdir $ArchDir -Force | Out-Null

        Write-Verbose $ArchDir
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
        if (! $Version) {
            $exe64 = '.\target\release\uwscr.exe'
            $Version = Get-BinaryVersion -BinPath $exe64
        }
        # cargo wix --nocapture
        candle -dProfile=release -dVersion="${Version}" -dPlatform=x64 -ext WixUtilExtension -o target/wix/x64.wixobj wix/x64.wxs -nologo | Out-Null
        $msipath = $Checkimg ?
                    ".release/${Version}/uwscr-${Version}-chkimg-x64.msi" :
                    ".release/${Version}/uwscr-${Version}-x64.msi"
        light -spdb -ext WixUIExtension -ext WixUtilExtension -cultures:ja-JP -out $msipath target/wix/x64.wixobj -nologo | Out-Null
        Get-Item $msipath
    }
    # x86
    if ("x86" -in $Architecture) {
        if (! $Version) {
            $exe86 = '.\target\i686-pc-windows-msvc\release\uwscr.exe'
            $Version = Get-BinaryVersion -BinPath $exe86
        }
        # cargo wix --compiler-arg "-dProfile=i686-pc-windows-msvc\release -dPlatform=x86" --nocapture
        candle -dProfile=i686-pc-windows-msvc\release -dVersion="${Version}" -dPlatform=x86 -ext WixUtilExtension -o target/wix/x86.wixobj wix/x86.wxs -nologo | Out-Null
        $msipath = ".release/${Version}/uwscr-${Version}-x86.msi"
        light -spdb -ext WixUIExtension -ext WixUtilExtension -cultures:ja-JP -out $msipath target/wix/x86.wixobj -nologo | Out-Null
        Get-Item $msipath
    }
}

if ($Schema) {
    $bin = @('.\target\release\uwscr.exe', '.\target\i686-pc-windows-msvc\release\uwscr.exe')
    $bin | ForEach-Object {
        if (Test-Path $_) {
            if (! $Version) {
                $Version = Get-BinaryVersion -BinPath $_
            }
            $path = ".release/${Version}/"
            & $_ --schema $path | Out-Null
            Join-Path $path "uwscr-settings-schema.json" | Get-Item
            break
        }
    }
}

if ($Document) {
    if ($Clean) {
        Invoke-Expression -Command '.\documents\make.bat clean'
    }
    $nojekyll = '.\documents\build\html\.nojekyll'
    if (!(Test-Path $nojekyll)) {
        $file = New-Item -Path $nojekyll -ItemType File
        $file.Attributes += 'ReadOnly'
    }
    Invoke-Expression -Command '.\documents\make.bat html'
}