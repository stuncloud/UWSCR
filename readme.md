UWSC互換スクリプト実行ツール UWSCR
=====

UWSCスクリプトを読み取り実行させるためのツールです

動作保証OS
----

Windows 10以上

導入方法
----

[最新版のリリースページ](https://github.com/stuncloud/UWSCR/releases/latest/#:~:text=Assets)下部のAssetsからzipファイルをダウンロードし、中の`uwscr.exe`を任意のフォルダに展開してください

- UWSCRx64.zip
    - 64ビット版uwscr
- UWSCRx86.zip
    - 32ビット版uwscr
- UWSCRx64_chkimg.zip
    - 64ビット版で`chkimg`が使えるもの
    - opencvのdllが別途必要です ([導入方法](https://github.com/stuncloud/UWSCR/wiki/%E3%82%A6%E3%82%A3%E3%83%B3%E3%83%89%E3%82%A6%E6%93%8D%E4%BD%9C%E9%96%A2%E6%95%B0#CHKIMG))

実行方法
----

現時点ではコマンドプロンプトやPowerShellからの実行を推奨します

### スクリプトの実行

```powershell
uwscr path\to\script.uws
```

### REPL

```powershell
uwscr -r
uwscr --repl
uwscr --repl path\to\script.uws # スクリプトの事前読み込み
```

### バージョン確認

```powershell
uwscr --version
```

### ヘルプ表示

```powershell
uwscr -h
uwscr --help
```

### オンラインヘルプ (wiki)

```powershell
uwscr -o
uwscr --online-help
```

詳細な使い方
----

[Wiki](https://github.com/stuncloud/UWSCR/wiki)のヘルプを参照してください

お問い合わせ
----

UWSCRに関する問い合わせはこちら

- [Discord](https://discord.gg/Y9VtAMZ)
- [UWSC仮掲示板](http://www3.rocketbbs.com/601/siromasa.html)

開発支援
----

以下からご支援いただけます

[UWSCR開発支援 - CAMPFIRE (キャンプファイヤー)](https://community.camp-fire.jp/projects/view/336074)