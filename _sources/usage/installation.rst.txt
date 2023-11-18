インストール方法
================

zipファイルダウンロード
-----------------------

| githubからダウンロードできます

最新版
++++++

1. githubの `最新リリース <https://github.com/stuncloud/UWSCR/releases/latest>`_ を開きます
2. そのリリースのAssetsから ``UWSCRx64.zip`` または ``UWSCRx86.zip`` をダウンロードします

任意のバージョン
++++++++++++++++

1. githubの `リリース一覧 <https://github.com/stuncloud/UWSCR/releases>`_ からダウンロードしたいバージョンのリリースを探します
2. そのリリースのAssetsから ``UWSCRx64.zip`` または ``UWSCRx86.zip`` をダウンロードします

zipファイルからのインストール
+++++++++++++++++++++++++++++

1. 任意のフォルダにzipファイルを展開します

.. admonition:: 展開されるファイル
    :class: note

    | 以下のファイルが展開されます

    - uwscr.exe

2. 必要に応じてファイルを展開したフォルダをPATH環境変数に登録します

wingetコマンド
--------------

| wingetコマンドによるインストールに対応しています
| wingetのバージョン1.4.11071以降からご利用いただけます

1. コマンドプロンプト、またはPowerShellで以下のいずれかのコマンドを実行します

.. sourcecode:: powershell

    winget install UWSCR
    # または
    winget install --id stuncloud.uwscr

2. 初回インストールの場合は ``uwscr.exe`` がインストールされたパスが環境変数PATHに登録されます
3. 必要に応じてコマンドプロンプトまたはPowerShellを再起動します
4. UWSCRが実行できることを確認します

.. sourcecode:: powershell

    # UWSCRのバージョンを表示
    cmd /c uwscr --version

.. admonition:: wingetコマンドが使えない場合
    :class: hint

    | 以下の方法でwingetをインストールします

    1. Microsoft Storeアプリを開きます
    2. **アプリ インストーラー** を検索します (``アプリインストーラー`` や ``app installer`` で見つかります)
    3. **アプリ インストーラー** をインストールしてください
    4. コマンドプロンプトやPowerShellで ``winget`` コマンドが使えることを確認します

    .. sourcecode:: powershell

        # wingetのバージョンを表示
        winget --version

.. admonition:: 最新リリースがwingetで公開されるタイミングについて
    :class: caution

    | wingetリポジトリへの登録申請はgithubでのリリース後に行われます
    | 登録には数日を要すため、githubでのリリース直後はwingetによるインストールが行えません
    | また、何かしらの理由により登録申請自体を行わない場合がありえます

    - 登録申請が通らなかった場合
    - 登録申請を行えなかった場合