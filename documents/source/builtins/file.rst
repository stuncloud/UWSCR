ファイル操作関数
================

テキストファイル
----------------

.. function:: fopen(ファイルパス, [モード=F_READ, 追記文字列=EMPTY])

    | テキストファイルを開きます

    .. admonition:: UWSCとの違い
        :class: caution

        | 戻り値のファイルIDが数値ではなくなりました

    :param 文字列 ファイルパス: 開きたいテキストファイルのパス
    :param 定数 省略可 モード: どのようにファイルを開くかを指定、 ``OR`` 連結可

        .. object:: F_EXISTS

            | パスの存在確認 (ディレクトリの場合は末尾に ``\`` を付ける)
            | 戻り値が真偽値になる

        .. object:: F_READ

            | ファイルを読む (SJIS、UTF8、UTF16対応)

        .. object:: F_WRITE

            | ``F_READ`` と併記しない場合はファイルを上書きする (UTF-8)
            | ``F_READ`` と併記した場合は元ファイルのエンコーディングを維持する

        .. object:: F_WRITE1

            | SJISで書き込む

        .. object:: F_WRITE8

            | UTF-8で書き込む

        .. object:: F_WRITE8B

            | BOM付きUTF-8で書き込む

        .. object:: F_WRITE16

            | UTF-16LEで書き込む

        .. object:: F_APPEND

            | 文末に追記し、即ファイルを閉じる
            | `追記文字列` を必ず指定する
            | ``F_WRITE`` 系と併記で書き込む文字列のエンコーディングを指定できる
            | 戻り値が書き込んだバイト数になる

        .. object:: F_NOCR

            | 文末に改行を入れない

        .. object:: F_TAB

            | CSVセパレータをカンマではなくタブ文字にする

        .. object:: F_EXCLUSIVE

            | 排他モードでファイルを開く

    :param 文字列 省略可 追記文字列: ``F_APPEND`` 指定時に追記する文字列

    :return: モードによる

        .. object:: 真偽値

            ``F_EXISTS`` 指定時、ファイルまたはディレクトリが存在する場合はTRUE

        .. object:: 数値

            ``F_APPEND`` 指定時、書き込んだバイト数

        .. object:: ファイルID

            ``F_EXISTS``, ``F_APPEND`` 以外を指定した場合、開いたファイルを示すIDを返す

.. function:: fget(ファイルID, 行, [列=0, ダブルクォート無視=FALSE])

    | ファイルを読み取ります

    .. admonition:: 使用条件
        :class: note

        | ``F_READ`` を指定してファイルを開く必要があります

    :param ファイルID ファイルID: ``fopen`` で開いたファイルのID
    :param 数値 行: 読み取る行の番号 (1から)
    :param 数値 列: 読み取るcsv列の番号 (1から)、0の場合行全体
    :param 真偽値 省略可 ダブルクォート無視: 列が1以上 (csv読み取り) の場合に有効

        .. object:: TRUE

            | ダブルクォートを無視する

        .. object:: FALSE

            | ダブルクォートで括られていたら単語と判断する
            | ダブルクォートはダブルクォートでエスケープする (``""``)

    :return: 読み取った文字列

    .. admonition:: サンプルコード

        | test.csv

        .. sourcecode:: none

            foo,bar,baz
            foo   ,    bar   ,  baz
            "ダブルクォートありのカラム","ダブルクォートの""エスケープ""",""
            "改行を
            含む","ここはまだ4行目"

        | スクリプト

        .. sourcecode:: uwscr

            fid = fopen("test.csv", F_READ)

            print fget(fid, 1) // foo,bar,baz
            print fget(fid, 1, 1) // foo
            // 前後のホワイトスペースはトリムされる
            print fget(fid, 2, 1) // 「    foo   」にはならず「foo」が返る
            // ダブルクォートで括られたカラム
            print fget(fid, 3, 1, FALSE) // ダブルクォートありのカラム
            print fget(fid, 3, 1, TRUE)  // "ダブルクォートありのカラム"
            // 第4引数FALSEはUWSCにおける 2 の動作が標準になりました
            print fget(fid, 3, 2, FALSE) // ダブルクォートの"エスケープ"
            print fget(fid, 3, 2, TRUE)  // "ダブルクォートの""エスケープ"""
            // 改行を含むカラムのサポート
            // 第4引数がFALSEかつ改行を含む文字列がダブルクォートで括られている場合のみ
            print fget(fid, 4, 1, FALSE) // 改行を
                                        // 含む
            print fget(fid, 4, 2, FALSE) // ここはまだ4行目

            fclose(fid)

.. function:: fput(ファイルID, 値, [行=0, 列=0])

    | ファイルに書き込みます

    .. admonition:: 使用条件
        :class: note

        | ``F_WRITE`` 系を指定してファイルを開く必要があります

    :param ファイルID ファイルID: ``fopen`` で開いたファイルのID
    :param 文字列 値: 書き込む文字列
    :param 数値 省略可 行: 書き込む行を指定

        .. object:: 0

            文末に新たな行として書き加えます

        .. object:: 1以上

            指定行に書き込みます (上書き)

        .. object:: F_ALLTEXT (定数)

            指定行に書き込みますが、ファイル全体を書き込む値で上書きします

    :param 数値 省略可 列: 書き込むCSV列を指定

        .. object:: 0

            行全体に書き込み

        .. object:: 1以上

            CSVカラムとして書き込み

        .. object:: F_INSERT (定数)

            | 指定した行へ上書きではなく挿入します
            | ``F_READ`` が未指定の場合無視されます

    :return: なし

.. function:: fdelline(ファイルID, 行)

    | 指定行を削除します

    .. admonition:: 使用条件
        :class: note

        | ``F_READ`` および ``F_WRITE`` 系を指定してファイルを開く必要があります

    :param ファイルID ファイルID: ``fopen`` で開いたファイルのID
    :param 数値 行: 削除する行の番号 (1から)、該当行がない場合なにもしない
    :return: なし

.. function:: fclose(ファイルID, [エラー抑止=FALSE])

    | ファイルを閉じて変更を適用します

    .. admonition:: ファイルの更新について
        :class: hint

        | ファイルを閉じない限り ``fput`` や ``fdelline`` による変更はファイルに反映されません

    :param ファイルID ファイルID: ``fopen`` で開いたファイルのID
    :param 真偽値 省略可 エラー抑止: TRUEにするとファイル書き込み時のエラーを無視する
    :return: ファイルへの書き込みが行われ正常に閉じられた場合はTRUE

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // 読み取り
            fid = fopen(path) // fopen(path, F_READ) と同等
            print fget(fid, 1)
            fclose(fid)

            // 書き込み
            fid = fopen(path, F_WRITE)
            fput(fid, text)
            fclose(fid) // 上書きされる

            // 読み書き
            fid = fopen(path, F_READ or F_WRITE)
            print fget(fid, 1)
            fput(fid, text)
            fclose(fid) // 編集して保存

            // エンコーディングを変更して保存
            fid = fopen(path, F_WRITE1) // SJISでファイルを書き込み
            fput(fid, text1)
            fclose(fid)

            fid = fopen(path, F_READ or F_WRITE16)
            fput(fid, text2)
            fclose(fid) // 編集してUTF-16で保存

            // 追記
            fopen(path, F_APPEND or F_WRITE16, text) // UTF-16で末尾に追記
            fopen(path, F_APPEND) // エラー; F_APPEND指定時は第三引数が必須

            // 自動ファイルクローズ
            print fget(fopen(path, F_READ), F_ALLTEXT)
            // ファイル識別子を変数に代入しなかった場合は読み書き関数実行後に自動でファイルが閉じられます

iniファイル
-----------

.. function:: readini([セクション=EMPTY, キー=EMPTY, ファイル="<#GET_UWSC_NAME>.ini"])

    | iniファイルを読み込みます

    :param 文字列 省略可 セクション: 読み出したいキーのあるセクション名を指定、省略時はセクション一覧を得る
    :param 文字列 省略可 キー: 値を読み出したいキーの名前を指定、省略時はキー一覧を得る
    :param 文字列またはファイルID 省略可 ファイル: 読み出すiniファイルのパス、またはファイルID

        .. admonition:: ファイルIDを利用する場合
            :class: note

            | ``F_READ`` を含めてfopenしている必要があります

    :return:

        .. object:: セクション省略時

            | iniファイルのセクション一覧を格納した配列
            | セクション省略時のキー指定は無視されます

        .. object:: キーを省略

            指定セクションのキー一覧を格納した配列

        .. object:: セクションとキーを指定

            | 該当キーの値
            | 該当キーが存在しない場合EMPTY

    .. admonition:: サンプルコード

        test.ini

        .. code:: ini

            [section]
            key1="あ"
            key2="い"
            key3="う"
            [foo]
            name="foo"
            [bar]
            name="bar"
            [baz]
            name="baz"

    スクリプト

    .. sourcecode:: uwscr

        ini = 'test.ini'
        print readini('foo', 'name', ini) // foo

        // セクションを省略(またはEMPTY指定)するとセクション一覧を取得
        print readini( , , ini) // [ section, foo, bar, baz ]
        print readini( , 'name', ini) // ↑と同じ結果 (セクション省略時のキーは無視される)

        // セクションを指定してキーを省略(またはEMPTY指定)するとキー一覧を収録
        print readini('section', , ini) // [ key1, key2, key3 ]

.. function:: writeini(セクション, キー, 値, [ファイル="<#GET_UWSC_NAME>.ini"])

    | iniファイルに書き込みます

    :param 文字列 セクション: 書き込みたいキーのあるセクション名、存在しない場合新規に作成されます
    :param 文字列 キー: 書き込みたいキーの名前、存在しない場合新規に作成されます
    :param 文字列 値: 該当キーに書き込む値
    :param 文字列またはファイルID 省略可 ファイル: 書き込むiniファイルのパス、またはファイルID

        .. admonition:: ファイルIDを利用する場合
            :class: note

            | ファイルIDは ``F_READ`` 及び ``F_WRITE`` 系を含めてfopenしている必要があります
            | また、ファイルIDを渡した場合はfcloseを呼ぶまで変更が反映されません

    :return: なし

.. function:: deleteini(セクション, [キー=EMPTY, ファイル="<#GET_UWSC_NAME>.ini"])

    | 指定キーまたはセクションを削除します

    :param 文字列 セクション: 削除したいキーのあるセクション名
    :param 文字列 キー: 削除したいキーの名前
    :param 文字列またはファイルID 省略可 ファイル: 書き込むiniファイルのパス、またはファイルID

        .. admonition:: ファイルIDを利用する場合
            :class: note

            | ファイルIDは ``F_READ`` 及び ``F_WRITE`` 系を含めてfopenしている必要があります
            | また、ファイルIDを渡した場合はfcloseを呼ぶまで変更が反映されません

    :return: なし

INI関数のファイルID利用について
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

| iniファイルをfopenで開き、そのファイルIDを各種ini関数に渡すことでiniファイルの読み書きができるようになりました

.. admonition:: サンプルコード

    .. sourcecode:: uwscr

        fid = fopen("hoge.ini", F_READ or F_WRITE)
        // ファイルパスの代わりにファイルIDを指定
        print readini("hoge", "fuga", fid)        // 読む場合はF_READが必要
        writeini("hoge", "fuga", "fugafuga", fid) // 書き込みにはF_READ or F_WRITEが必要
        deleteini("hoge", "fuga", fid)            // 削除にもF_READ or F_WRITEが必要

        fclose(fid) // iniファイルへの書き込みが反映される

| 以下のような用途を想定しています

- 同一iniファイルへの複数回の読み書きを行う場合にファイルアクセスを減らしたい
- iniファイル編集時に排他制御(``F_EXCLUSIVE``)したい

その他のファイル操作
--------------------

.. function:: deletefile(ファイルパス)

    | ファイルを削除します
    | ``*``, ``?`` によるワイルドカード指定も可能

    :param 文字列 ファイルパス: 削除したいファイルのパス
    :return: 削除成功時TRUE

.. function:: getdir(ディレクトリパス, [フィルタ="*", 非表示ファイル=FALSE, 取得順=ORDERBY_NAME])

    | 対象ディレクトリに含まれるファイル、またはディレクトリの一覧を取得します

    :param 文字列 ディレクトリパス: 対象ディレクトリのパス
    :param 文字列 省略可 フィルタ:

        | ファイル名のフィルタ、ワイルドカード(``*``, ``?``)可
        | ``\`` のみ、または ``\`` から始まる文字列指定でファイルではなくディレクトリ一覧を返す

    :param 真偽値 省略可 非表示ファイル: 非表示ファイルを含めるかどうか
    :param 定数 省略可 取得順: 取得順を示す定数

        .. object:: ORDERBY_NAME

            ファイル名順

        .. object:: ORDERBY_SIZE

            サイズ順

        .. object:: ORDERBY_CREATED

            作成日時順

        .. object:: ORDERBY_MODIFIED

            更新日時順

        .. object:: ORDERBY_ACCESSED

            最終アクセス日時順


    :return: 該当するファイル名またはディレクトリ名の一覧を格納した配列

        .. admonition:: UWSCとの違い
            :class: caution

            | 該当ファイルの個数ではなく配列が返るようになりました
            | それに伴い特殊変数 ``GETDIR_FILES`` は廃止されました

    .. admonition:: サンプルコード

        | ファイル構成

        .. code::

            C:\test\
            ├ foo1.txt
            ├ foo2.txt
            ├ bar.txt
            ├ baz.txt
            ├ hidden.txt (隠しファイル)
            ├ dir1\
            ├ dir2\
            ├ folder1\
            └ folder2\

        | スクリプト

        .. sourcecode:: uwscr

            // ファイル一覧の表示
            print getdir('C:\test') // [foo1.txt, foo2.txt, bar.txt, baz.txt]
            // ファイル名のフィルタ
            print getdir('C:\test', 'foo*') // [foo1.txt, foo2.txt]
            // 隠しファイルも表示
            print getdir('C:\test', , TRUE) // [foo1.txt, foo2.txt, bar.txt, baz.txt, hidden.txt]
            // フォルダ一覧の表示
            print getdir('C:\test', '\') // [dir1, dir2, folder1, folder2]
            // フォルダ一名のフィルタ
            print getdir('C:\test', '\dir*') // [dir1, dir2]


.. .. function:: dropfile(ウィンドウID, ディレクトリパス, ファイル名, [ファイル名, ...])
.. .. function:: dropfile(ウィンドウID, x, y, ディレクトリパス, ファイル名, [ファイル名, ...])
..     :noindex:

    | 対象ウィンドウにファイルをドロップします

    :param 数値 ウィンドウID: 対象ウィンドウのID
    :param 数値 x: ドロップしたい位置のX座標 (ウィンドウに対する相対座標、省略時は中央)
    :param 数値 y: ドロップしたい位置のY座標 (ウィンドウに対する相対座標、省略時は中央)
    :param 文字列 ディレクトリパス: ドロップしたいファイルのあるディレクトリのパス
    :param 文字列 ファイル名: ドロップするファイル名、10個まで指定可 (x, y指定時は8個まで)

        .. hint::

            | ファイル名の配列を渡すこともできます
            | 一度に10個以上のファイルをドロップしたい場合など

    :return: なし

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            id = getid( , "CabinetWClass") // Explorer

            dropfile(id, "C:\test", "foo.txt") // C:\test\foo.txtをドロップ
            dropfile(id, "C:\test", "foo.txt", "bar.txt", "baz.txt") // 10個まで指定可能

            dropfile(id, "C:\test", ["foo.txt", "bar.txt", "baz.txt"]) // 配列指定も可能
            bmps = getdir("C:\test", "*.bmp")
            dropfile(id, "C:\test", bmps) // getdirで得たファイル名を渡す

            // 第二第三引数が数値なら対象ウィンドウの相対座標として扱いその位置へドロップする
            dropfile(id, 100, 100, "C:\test", "foo.txt")


ZIPファイル
-----------

.. function:: zip(zipファイル, ファイル, [ファイル, ...])

    | zipファイルを作成します

    :param 文字列 zipファイル: 作成するzipファイルのパス
    :param 文字列または配列 ファイル:

        | zipファイルに含めたいファイルのパス (10個まで)
        | パスの配列を渡すこともできる

    :return: 成功時TRUE

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            files = [
                'foo.uws',
                'bar.uws',
                'baz.uws',
                'modules\qux.uws',
                'modules\quux.uws'
            ]

            zip("test.zip", files)

.. function:: unzip(zipファイル, 展開先フォルダ)

    | zipファイルを指定フォルダに展開します
    | 展開先フォルダが存在しない場合は新規に作成されます
    | すでに同名ファイルが存在する場合は上書きされます

    :param 文字列 zipファイル: 展開したいzipファイルのパス
    :param 文字列 展開先フォルダ: 展開先フォルダのパス
    :return: 成功時TRUE

        .. hint:: 失敗した場合でも一部のファイルが展開されることがあります

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            unzip("test.zip", "out")

            for file in getdir("out")
                print file
            next
            for dir in getdir('out', '\')
                for file in getdir("out\<#dir>")
                    print "<#dir>/<#file>"
                next
            next
            // foo.uws
            // bar.uws
            // baz.uws
            // modules\qux.uws
            // modules\quux.uws

.. function:: zipitems(zipファイル)

    | zipファイルに含まれるファイル一覧を取得します

    :param 文字列 zipファイル: zipファイルのパス
    :return: ファイル名を格納した配列 (フォルダの区切りは ``/``)

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            for item in zipitems("test.zip")
                print item
            next
            // foo.uws
            // bar.uws
            // baz.uws
            // modules/qux.uws
            // modules/quux.uws
