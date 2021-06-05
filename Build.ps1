[CmdletBinding()]
param(
    [Parameter(Mandatory=$false)]
    [string] $Version,
    [switch] $Release,
    [string] $OutDir = '.\.release',
    [switch] $Installer,
    [ValidateSet("both","x64","x86")]
    [string] $Arch = "both"
)

# リリースビルドの場合vcのライブラリをスタティックリンクする
if ($Release) {
    $env:RUSTFLAGS='-C target-feature=+crt-static'
} else {
    $env:RUSTFLAGS=''
}

if (! $Installer -or ($Release -and $Installer)) {
    # build x64 exe
    $cmd = 'cargo build {0}' -f $(if ($Release) {'--release'})
    Invoke-Expression -Command $cmd
    # build x86 exe
    $cmd = 'cargo build --target=i686-pc-windows-msvc {0}' -f $(if ($Release) {'--release'})
    Invoke-Expression -Command $cmd
}

if ($Release) {
    $env:RUSTFLAGS=''

    $exe64 = '.\target\release\uwscr.exe'
    $exe86 = '.\target\i686-pc-windows-msvc\release\uwscr.exe'
    $exe64, $exe86 | ForEach-Object {
        if (! (Test-Path $_)) {
            Write-Error "$($_) が見つかりません"
            break
        }
    }
    if (! $Version) {
        if (('{0} --version' -f $exe64 | Invoke-Expression) -match '\d+\.\d+\.\d+') {
            $Version = $Matches[0]
        } else {
            Write-Error "uwscrのバージョンが不明"
            break
        }
    }
    if (! (Test-Path($OutDir))) {
        mkdir $OutDir | Out-Null
    }
    $verpath = Join-Path -Path $OutDir -ChildPath $Version
    $x64path = Join-Path -Path $verpath -ChildPath 'x64'
    $x86path = Join-Path -Path $verpath -ChildPath 'x86'
    if (! (Test-Path $verpath)) {
        mkdir $verpath | ForEach-Object {
            mkdir $x64path | Out-Null
            mkdir $x86path | Out-Null
        }
    }
    $exe64 | Copy-Item -Destination $x64path
    $exe86 | Copy-Item -Destination $x86path
    $64zip = Join-Path -Path $verpath -ChildPath UWSCRx64.zip
    $86zip = Join-Path -Path $verpath -ChildPath UWSCRx86.zip
    Get-ChildItem $exe64 | Compress-Archive -DestinationPath $64zip -Force
    Get-Item $64zip
    Get-ChildItem $exe86 | Compress-Archive -DestinationPath $86zip -Force
    Get-Item $86zip
}

# msi installer
if ($Installer) {
    # requires wix toolset
    if (! (Get-Command candle,light -ea SilentlyContinue | Where-Object Source -Match 'WiX Toolset')) {
        Write-Warning "WiX Toolsets not found"
        break;
    }

    $exe64 = '.\target\release\uwscr.exe'
    $exe86 = '.\target\i686-pc-windows-msvc\release\uwscr.exe'
    $exe64, $exe86 | ForEach-Object {
        if (! (Test-Path $_)) {
            Write-Error "$($_) が見つかりません"
            break
        }
    }
    # x64 for default
    if ($Arch -in @("both","x64")) {
        if (('{0} --version' -f $exe64 | Invoke-Expression) -match '\d+\.\d+\.\d+') {
            $Version = $Matches[0]
        } else {
            Write-Error "uwscrのバージョンが不明"
            break
        }
        # cargo wix --nocapture
        candle -dProfile=release -dVersion="${Version}" -dPlatform=x64 -ext WixUtilExtension -o target/wix/x64.wixobj wix/x64.wxs -nologo | Out-Null
        $msipath = ".release/${Version}/uwscr-${Version}-x64.msi"
        light -spdb -ext WixUIExtension -ext WixUtilExtension -cultures:ja-JP -out $msipath target/wix/x64.wixobj -nologo | Out-Null
        Get-Item $msipath
    }
    # x86
    if ($Arch -in @("both","x86")) {
        if (('{0} --version' -f $exe86 | Invoke-Expression) -match '\d+\.\d+\.\d+') {
            $Version = $Matches[0]
        } else {
            Write-Error "uwscrのバージョンが不明"
            break
        }
        # cargo wix --compiler-arg "-dProfile=i686-pc-windows-msvc\release -dPlatform=x86" --nocapture
        candle -dProfile=i686-pc-windows-msvc\release -dVersion="${Version}" -dPlatform=x86 -ext WixUtilExtension -o target/wix/x86.wixobj wix/x86.wxs -nologo | Out-Null
        $msipath = ".release/${Version}/uwscr-${Version}-x86.msi"
        light -spdb -ext WixUIExtension -ext WixUtilExtension -cultures:ja-JP -out $msipath target/wix/x86.wixobj -nologo | Out-Null
        Get-Item $msipath
    }
}
