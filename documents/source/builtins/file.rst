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

        .. object:: F_AUTOCLOSE

            | ファイルIDが破棄された際に自動でファイルをクローズする

            .. admonition:: 自動クローズについて
                :class: note

                | ファイルIDオブジェクトの参照がすべて失われた場合に自動クローズ処理が実施されます

                .. sourcecode:: uwscr

                    // fopenが返すファイルIDをfputが処理した直後にファイルIDが失われるため自動クローズされhogehogeが書き込まれる
                    fput(fopen("hgoe.txt", F_WRITE or F_AUTOCLOSE), "hogehoge")

                    fid = fopen("fuga.txt", F_WRITE or F_AUTOCLOSE)
                    fput(fid, "fugafuga")
                    fid = EMPTY // この時点でfuga.txtが閉じられる

                    fid = fopen("piyo.txt", F_WRITE or F_AUTOCLOSE)
                    // ファイルIDのコピー
                    fid2 = fid
                    fput(fid, "fugafuga")
                    fid = EMPTY // この時点ではまだpiyo.txtは閉じられない
                    msgbox(1)
                    fid2 = EMPTY // ここでpiyo.txtが閉じられる

    :param 文字列 省略可 追記文字列: ``F_APPEND`` 指定時に追記する文字列

    :return: モードによる

        .. object:: 真偽値

            ``F_EXISTS`` 指定時、ファイルまたはディレクトリが存在する場合はTRUE

        .. object:: 数値

            ``F_APPEND`` 指定時、書き込んだバイト数

        .. object:: ファイルID

            ``F_EXISTS``, ``F_APPEND`` 以外を指定した場合、開いたファイルを示すIDを返す

    .. admonition:: ファイルが開けない場合の動作について
        :class: note

        | UWSCでは-1を返していましたが、UWSCRでは実行時エラーとなりファイルが開けない理由を明確にします。
        | 例として、以下のような状況でエラーとなります

        - ``F_READ`` のみを指定し存在しないファイルを開こうとした場合 (読み出すファイルが無いため)
        - ``F_WRITE`` が含まれていて、読み取り専用のファイルを開こうとした場合 (書き込めないため)

.. function:: fget(ファイルID, 行, [列=0, ダブルクォート無視=FALSE])

    | ファイルを読み取ります

    .. admonition:: 使用条件
        :class: note

        | ``F_READ`` を指定してファイルを開く必要があります

    :param ファイルID ファイルID: ``fopen`` で開いたファイルのID
    :param 数値 行: 読み取る行の番号、または以下の定数を指定 (定数指定時は以降の引数は無視される)

        .. object:: F_LINECOUNT

            ファイルの行数を返す

        .. object:: F_ALLTEXT

            ファイル全体のテキストを返す

    :param 数値 列: 読み取るcsv列の番号 (1から)、0の場合は行全体
    :param 真偽値または2 省略可 ダブルクォート無視: 列が1以上 (csv読み取り) の場合に有効

        .. object:: FALSE

            | 両端のダブルクォートを削除する

        .. object:: TRUE

            | 両端にダブルクォートがあってもなにもしない

        .. object:: 2

            | 連続するダブルクォート (``""``) を単一のダブルクォート (``"``) にする
            | その後両端のダブルクォートを削除する (FALSEと同じ処理)

    :return:

        | 読み取った文字列
        | 該当行または列が存在しない場合は ``EMPTY``

        .. admonition:: EMPTYについて
            :class: note

            | UWSCでは指定行および列が存在しない場合に空文字(``""``)を返していましたが、UWSCRでは ``EMPTY`` を返すように変更しています
            | これにより空文字を読み取った場合と、不正な行や列を読み取った場合を区別できるようになりました

    .. admonition:: サンプルコード

        | test.csv

        .. sourcecode:: none

            foo,bar,baz
            foo   ,    bar   ,  baz
            "ダブルクォートありのカラム","ダブルクォートの""エスケープ""",""

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

            ファイル全体を書き込む値で上書きします

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
            print fput(fopen(path, F_WRITE or F_AUTOCLOSE), "auto close")
            // F_AUTOCLOSEによりfput実行後にファイルが自動でクローズされる

CSVファイル
-----------

.. function:: csvopen(CSVパス, [ヘッダ有無=FALSE, TSVモード=FALSE])

    | CSVファイルを開く

    :param 文字列 CSVパス: CSVファイルのパス
    :param 真偽値 省略可 ヘッダ有無: 対象CSVファイルにヘッダ行があるかどうか
    :param 真偽値または文字 省略可 TSVモード: FALSEの場合はカンマ区切り、TRUEにするとタブ文字区切り、または任意のASCII文字
    :rtype: CSVオブジェクト
    :return: CSVファイルを示すオブジェクト、各種csv関数で利用される

.. function:: csvclose(csv)

    | 編集したCSVをファイルに書き出す
    | バッファに変更があった場合のみ対象ファイルに書き込みを行う
    | ``csvopen`` で指定したファイルが存在しない場合は新しいファイルが作成される
    | この関数呼び出し後のCSVオブジェクトに対して再度この関数を実行しても書き込みは行われない

    .. admonition:: 自動クローズ
        :class: hint

        | CSVオブジェクトが破棄された場合は自動でこの関数と同等の処理が行われます
        | (``fopen`` の ``F_AUTOCLOSE`` 指定時と同様です)

    .. admonition:: クローズ後のCSVオブジェクトについて
        :class: note

        | バッファに対する読み書きはできますが、再度 ``csvclose`` で書き込みを行うことはできません

    :param CSVオブジェクト csv: ``csvopen`` の戻り値
    :return: なし

.. function:: csvread(csv, [行, 列])

    | CSVバッファから値を読み出します

    :param CSVオブジェクト csv: ``csvopen`` の戻り値
    :param 数値 省略可 行: CSVの行番号
    :param 数値または文字列 省略可 列: CSVの列番号、またはヘッダのカラム名
    :rtype: 文字列または配列
    :return: 行列の指定方法により得られる値が変わります

        .. list-table::
            :header-rows: 1
            :align: left

            * - 行
              - 列
              - 値
            * - 省略
              - 省略
              - CSV全体の文字列
            * - 省略
              - 1以上
              - 該当列の配列
            * - 0
              - 省略
              - ヘッダ行の配列
            * - 1以上
              - 省略
              - 該当行の配列
            * - 1以上
              - 1以上
              - 該当行及び列の文字列

    .. admonition:: サンプルコード

        .. sourcecode::
            :caption: test.csv

            項目1,項目2,項目3
            1,2,3
            10,20,30
            100,200,300

        .. sourcecode:: uwscr

            // ヘッダ行を有効にして開く
            csv = csvopen("test.csv", true)

            // CSV全体を得る
            print csvread(csv)
            // 項目1,項目2,項目3
            // 1,2,3
            // 10,20,30
            // 100,200,300

            // ヘッダ行の配列を得る
            print csvread(csv, 0)          // [項目1, 項目2, 項目3]
            // 2行目の配列を得る
            print csvread(csv, 2)          // [10, 20, 30]
            // 2行目1列目の文字列を得る
            print csvread(csv, 2, 1)       // 10
            // 1列目の配列を得る
            print csvread(csv, , 1)        // [1, 10, 100]
            // 列をカラム名で指定
            print csvread(csv, 3, "項目2") // 200
            print csvread(csv, , "項目3")  // [3, 30, 300]

.. function:: csvwrite(csv, 行, 列, 値)

    | CSVバッファに書き込みを行う

    :param CSVオブジェクト csv: ``csvopen`` の戻り値
    :param 数値 行: CSVの行番号
    :param 数値または文字列 列: CSVの列番号、またはヘッダのカラム名
    :param 文字列または配列 値: 書き込む値
    :rtype: 真偽値
    :return: 書き込み時true

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            new_csv = "new.csv"
            deletefile(new_csv)

            // ファイルを新規作成
            csv = csvopen(new_csv, true)

            // 0行目指定でヘッダを書き込む
            csvwrite(csv, 0, 1, "項目1")
            csvwrite(csv, 0, 2, "項目2")
            csvwrite(csv, 0, 3, "項目3")
            // 指定位置に書き込み
            csvwrite(csv, 1, 1, "1-1")
            // 配列指定で複数列書き込み
            csvwrite(csv, 2, 1, ["2-1", "2-2", "2-3"])
            // 3行目を飛ばして4行目に書き込み
            csvwrite(csv, 4, 1, ["4-1", "4-2", "4-3"])
            // 2列目から書き込み
            csvwrite(csv, 5, 2, ["5-2", "5-3"])
            // 列の数は可変
            csvwrite(csv, 6, 1, ["6-1", "6-2", "6-3", "6-4", "6-5"])

            // 全体読み出し
            print csvread(csv)
            // 項目1,項目2,項目3
            // 1-1
            // 2-1,2-2,2-3
            // ""
            // 4-1,4-2,4-3
            // ,5-2,5-3
            // 6-1,6-2,6-3,6-4,6-5

            csvclose(csv) // 保存

            // 書き出したファイルも確認
            print fget(fopen(new_csv, F_READ or F_AUTOCLOSE), F_ALLTEXT)
            // 項目1,項目2,項目3
            // 1-1
            // 2-1,2-2,2-3
            // ""
            // 4-1,4-2,4-3
            // ,5-2,5-3
            // 6-1,6-2,6-3,6-4,6-5

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
    :return: 該当ファイルすべてを削除できた場合TRUE、一つでも該当ファイルが削除できなかった場合は該当ファイルが存在しない場合はFALSE

    .. admonition:: ワイルドカード指定時の動作について
        :class: caution

        | UWSCではワイルドカード指定時に削除できないファイルが含まれていたとしても別のファイルが一つでも削除できればTRUEを返していましたが、UWSCRでは一つでも削除できないファイルが含まれていればFALSEを返します

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

.. function:: dropfile(ID, ディレクトリ, ファイル名, [ファイル名...])

    | ファイルをウィンドウにドロップします
    | ドロップ位置はクライアント領域の中央です

    :param 数値 ID: ファイルをドロップするウィンドウのID
    :param 数値 ディレクトリ: ドロップするファイルの存在するディレクトリパス
    :param 文字列または配列 ファイル名: ファイル名を示す文字列、またはファイル名を示す文字列を含む配列変数
    :return: なし

.. function:: dropfile(ID, x, y, ディレクトリ, ファイル名, [ファイル名...])
    :noindex:

    | 第二、第三引数が数値だった場合はファイルのドロップ座標を指定します
    | 対象ウィンドウのクライアント座標を指定します

    :param 数値 x: クライアントX座標
    :param 数値 y: クライアントY座標

    .. admonition:: ファイル名指定数の下限および上限
        :class: hint

        | 上限は座標未指定時は34、座標指定時は32個まで (すべての引数の個数上限が36)
        | ファイル数がそれより多い場合は配列変数を使ってください
        | 下限は1です (最低1つ指定する必要がある)

    .. admonition:: マウス移動が行われます
        :class: caution

        | ドロップ処理時に瞬間的にマウスカーソルを指定座標に移動しています
        | (UWSCと同様の処理)

    .. admonition:: 実行要件
        :class: important

        | 対象ウィンドウが ``WM_DROPFILES`` メッセージを処理できる必要があります


ZIPファイル
-----------

.. function:: zip(zipファイル, ファイル, [ファイル, ...])

    | zipファイルを作成します

    :param 文字列 zipファイル: 作成するzipファイルのパス
    :param 文字列または配列 ファイル:

        | zipファイルに含めたいファイルのパス (10個まで)
        | パスの配列を渡すこともできる

        .. admonition:: 格納されるファイルのパス構成について
            :class: note

            | 指定したパスがファイルの場合はそのファイル名でzipに格納します
            | フォルダが指定された場合はそのフォルダ以下のすべてのファイルをフォルダからの相対パスでzipに格納します

    :return: 成功時TRUE

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            files = [
                'foo.uws',
                'bar.uws',
                'baz.uws',
                'modules', // フォルダ指定
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
            // modules\qux.uws
            // modules\quux.uws
