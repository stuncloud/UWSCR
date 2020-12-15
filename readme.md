UWSC互換スクリプト実行ツール UWSCR
=====

UWSCスクリプトを読み取り実行させるためのツールです

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

詳細な使い方
----

[Wiki](https://github.com/stuncloud/UWSCR/wiki)のヘルプを参照してください

ビルド方法
----

自分でソースからビルドしてみたい方向け

### Rust開発環境の準備

Windows 10 x64環境での手順を記載します

#### Visual C++ Build Tools のインストール

1. [Visual Studio 2019のツール](https://visualstudio.microsoft.com/ja/downloads/#vstool-2019-ja-family)からBuild Tools for Visual Studio 2019のインストーラをダウンロード
2. インストーラからVisual C++ Build Toolsのインストールを行う

#### Rustのインストール

1. [Rust をインストール - Rustプログラミング言語](https://www.rust-lang.org/ja/tools/install)から`rustup-init.exe`をダウンロード
2. PowerShellなどから`rustup-init.exe`を実行
3. プロンプトに従いインストールを完了する
4. `rustup --version` や `cargo --version` が正常に実行できればOK
    ※ 実行できない場合は一旦PowerShellなどを再起動してみてください
5. `rustup target install i686-pc-windows-msvc` を実行
6. `rustup show` を実行

    ```
    Default host: x86_64-pc-windows-msvc
    rustup home:  C:\Users\(your name)\.rustup

    installed targets for active toolchain
    --------------------------------------

    i686-pc-windows-msvc
    x86_64-pc-windows-msvc

    active toolchain
    ----------------

    stable-x86_64-pc-windows-msvc (default)
    rustc 1.47.0 (18bf6b4f0 2020-10-07)
    ```

    こんな感じになっていればOK

### ビルドする

1. uwscrを`git clone`し、PowerShellでそのディレクトリへ移動
2. `.\Build.ps1`を実行する
3. 以下が出力される
   - x64版
     - `.\target\debug\uwscr.exe`
   - x86版
     - `.\target\i686-pc-windows-msvc\debug\uwscr.exe`


お問い合わせ
----

UWSCRに関する問い合わせはこちら

- [Discord](https://discord.gg/Y9VtAMZ)
- [UWSC仮掲示板](http://www3.rocketbbs.com/601/siromasa.html)

開発支援
----

以下からご支援いただけます

[UWSCR開発支援 - CAMPFIRE (キャンプファイヤー)](https://community.camp-fire.jp/projects/view/336074)