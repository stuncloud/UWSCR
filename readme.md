# UWSC互換スクリプト実行ツール UWSCR

UWSCスクリプトを読み取り実行させるためのツールです

## 動作保証OS

Windows 10以上

## 導入方法

### リリースページからダウンロード

[最新版のリリースページ](https://github.com/stuncloud/UWSCR/releases/latest/#:~:text=Assets)下部のAssetsからzipファイルをダウンロードし、中の`uwscr.exe`を任意のフォルダに展開してください

- UWSCRx64.zip
    - 64ビット版uwscr
- UWSCRx86.zip
    - 32ビット版uwscr
    
### wingetによるインストール

winget (Windows Package Manager) を使ってUWSCRをインストールできます  
wingetがインストールされていない場合はMicrosoftストアにて[アプリインストーラー](https://www.microsoft.com/p/app-installer/9nblggh4nns1)をインストールしてください  
wingetはバージョン1.4.11071以上をご利用ください

```powershell
# バージョンを確認
PS> winget --version
v1.4.11071
```

以下のコマンドによりUWSCRがインストールされます

```powershell
winget install UWSCR
# または
winget install --id stuncloud.uwscr
```

`uwscr.exe` のインストール先は以下です

- `%LOCALAPPDATA%\Microsoft\WinGet\Links`

インストール時にこのパスがユーザー環境変数 `%PATH%` に登録されます  
実行環境(PowerShellやExplorer)の再起動を行うことでどこからでも `uwscr.exe` を実行できるようになります  
うまく行かない場合は再ログインするか、手動で `%PATH%` に追加登録してください

## 実行方法

### スクリプトの実行

```powershell
cmd /c uwscr path\to\script.uws
```

### REPL

```powershell
cmd /c uwscr -r
cmd /c uwscr --repl
cmd /c uwscr --repl path\to\script.uws # スクリプトの事前読み込み
```

### バージョン確認

```powershell
cmd /c uwscr --version
```

### ヘルプ表示

```powershell
cmd /c uwscr -h
cmd /c uwscr --help
```

### オンラインヘルプ

```powershell
cmd /c uwscr -o
cmd /c uwscr --online-help
```

## 詳細な使い方

[UWSCRオンラインヘルプ](https://stuncloud.github.io/UWSCR/index.html)を参照してください

## お問い合わせ

UWSCRに関する問い合わせはこちら

- [Discord](https://discord.gg/Y9VtAMZ)
- [Issue](https://github.com/stuncloud/UWSCR/issues)

## 開発支援

以下からご支援いただけます

[UWSCR開発支援 - CAMPFIRE (キャンプファイヤー)](https://community.camp-fire.jp/projects/view/336074)