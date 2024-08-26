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
    

## 実行方法

UWSCRはコンソールアプリケーションです  
コマンドプロンプトやPowerShell上で実行してください  
Explorer等から実行した場合はコンソールウィンドウが表示されます

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

### オンラインヘルプ

```powershell
uwscr -o
uwscr --online-help
```

## 詳細な使い方

[UWSCRオンラインヘルプ](https://stuncloud.github.io/UWSCR/index.html)を参照してください

## コーディング支援機能

UWSCRは[Language Server](https://stuncloud.github.io/UWSCR/usage/language_server.html)として動作します  
また、それを利用したVSCode向けの[拡張機能](https://github.com/stuncloud/vscode-uwscr/releases/latest)を公開しています

## ライセンス

UWSCRのソースコードは [MIT](https://github.com/stuncloud/UWSCR/blob/master/LICENSE) でライセンスされています  
依存crateのライセンスは [サードパーティライセンス](https://stuncloud.github.io/UWSCR/_static/license.html) を参照してください


## Q&A

### UWSCRはUWSCの正式な後継ソフトですか？

いいえ、違います  

UWSCの作者であるumiumi氏が消息不明になり、UWSCの将来に不安を感じたことが開発のきっかけとなっています  
なのでUWSCRはumiumi氏の意思とは無関係に作られたものであり、正式な後継ではありません

UWSCRはUWSCの動作をなるべく再現することを目的として、一から開発されたものです  
UWSCのソースコードの流用ができないため目に見える範囲を模倣することで作られています  
そのため目に見えぬ部分の完全再現はなかなかに難しく、同じ動作を保証できるまでには至っていません  

名前を寄せているのはUWSCへの敬意からであり、そしてRにはRespectであったり、Rebootであったりというような意味を込めています

### 現在UWSCを使っています、UWSCRに乗り換えるべきですか？

必ずしもその必要はありません  
UWSCの使用に不都合がなければそのままUWSCを使い続けてください  
(WindowsがWindowsである限りUWSCが動作しなくなるということはないです)  
しかし、例えば[ブラウザ操作](https://stuncloud.github.io/UWSCR/builtins/web.html)などUWSCのままでは不都合があるといった場合であればUWSCRの利用を考慮しても良いかもしれません  

### UWSCで使っていたスクリプトはそのまま動作しますか？

動作するものもあれば、しないものもあります  
それは仕様変更によるものであったり、不具合による場合もありえます  

仕様変更に関しては、その大半は作者がUWSCに感じていた不便さを解消するために行なわれています  
そのため仕様変更に不都合を感じる方もいらっしゃるかと思いますが、そこはご容赦いただきたく思います  
仕様の違いに関してはなるべく[ドキュメント](https://stuncloud.github.io/UWSCR/index.html)に記載しているので、それらを参考にしてくください

不具合またはその疑念がある場合は[issue](https://github.com/stuncloud/UWSCR/issues/new/choose)で報告していただけると助かります

### UWSCRを使うメリットはありますか？

UWSCの言語仕様に限界を感じていた方にはメリットがあるかもしれません
UWSCRはUWSCの書き味を残しつつも、拡張された書式により柔軟かつ高度なコーディングが行えるようになっています

- 値として扱えるようになった配列、配列リテラル表記
- 新たな演算子の追加
- classのインスタンス作成
- 無名関数、高階関数
- JSONサポート
- Cライク構造体のサポート
- dll関数のコールバック対応

など、UWSCにはなかった様々な改善が施されています


## お問い合わせ

UWSCRに関する問い合わせはこちら

- [Discord](https://discord.gg/Y9VtAMZ)
- [Issue](https://github.com/stuncloud/UWSCR/issues)

## 開発支援

以下からご支援いただけます

[UWSCR開発支援 - CAMPFIRE (キャンプファイヤー)](https://community.camp-fire.jp/projects/view/336074)