GUI
===

ダイアログ
----------

.. function:: msgbox(メッセージ, [ボタン種=BTN_OK, x=EMPTY, y=EMPTY, フォーカス=EMPTY])

    | メッセージボックスを表示します

    .. note:: メッセージボックスのクラス名は ``UWSCR.MsgBox`` です

    :param 文字列 メッセージ: ダイアログに表示するメッセージ
    :param ボタン定数 省略可 ボタン種: 表示するボタンを示す定数、 ``OR`` 連結で複数表示

        .. object:: BTN_YES

            はい

        .. object:: BTN_NO

            いいえ

        .. object:: BTN_OK

            OK

        .. object:: BTN_CANCEL

            キャンセル

        .. object:: BTN_ABORT

            中止

        .. object:: BTN_RETRY

            再試行

        .. object:: BTN_IGNORE

            無視

    :param 数値 省略可 x: ダイアログの初期表示位置のX座標を指定、省略時(EMPTY)なら画面中央
    :param 数値 省略可 y: ダイアログの初期表示位置のY座標を指定、省略時(EMPTY)なら画面中央

    .. hint:: x, yに-1を指定するとそれぞれ前回表示した位置になります

    :param ボタン定数 省略可 フォーカス: カーソルの初期位置をボタン定数で指定、省略時や該当ボタンがない場合は一番左のボタンがフォーカスされます

    :return: 押されたボタンを示すボタン定数 (×ボタンで閉じられた場合は ``BTN_CANCEL``)

.. function:: input(メッセージ, [デフォルト値=EMPTY, マスク表示=FALSE, x=EMPTY, y=EMPTY])

    | インプットボックスを表示します

    .. note:: インプットボックスのクラス名は ``UWSCR.Input`` です

    :param 文字列または配列 メッセージ:

        .. object:: 文字列

            メッセージ欄に表示されるメッセージ

        .. object:: 文字列の配列

            | 1番目がメッセージ欄に表示される
            | 2番目以降はラベルとして表示され、ラベル毎に入力欄が追加される
            | ラベルは最大5つまで

    :param 文字列または配列 省略可 デフォルト値:

        .. object:: 文字列

            入力欄に予め入力しておく値

        .. object:: 文字列の配列

            入力欄毎のデフォルト入力値

    :param 真偽値または配列 省略可 マスク表示:

        .. object:: 真偽値

            入力欄をマスク表示するかどうか

        .. object:: 真偽値の配列

            入力欄毎のマスク設定

    :param 数値 省略可 x: ダイアログの初期表示位置のX座標を指定、省略時(EMPTY)なら画面中央
    :param 数値 省略可 y: ダイアログの初期表示位置のY座標を指定、省略時(EMPTY)なら画面中央

    .. hint:: x, yに-1を指定するとそれぞれ前回表示した位置になります

    :return:

        .. object:: 入力欄が一つの場合

            入力された値、キャンセル時はEMPTY

        .. object:: 入力欄が複数の場合

            それぞれに入力された値の配列、キャンセル時は空配列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // ラベルを2つ指定し入力欄を2つにする
            labels = ['ログイン', 'ユーザー名', 'パスワード']
            // 1つ目の入力欄のみデフォルト値を入れる
            default = ['UserA', EMPTY]
            // 2つ目の入力欄がマスクされるようにする
            mask = [FALSE, TRUE]

            // 入力値は配列で返る
            user = input(labels, default, mask)
            print 'ユーザー名: ' + user[0]
            print 'パスワード: ' + user[1]

.. function:: slctbox(表示方法, タイムアウト秒, メッセージ=EMPTY, 表示項目, [表示項目2, ..., 表示項目31])
.. function:: slctbox(表示方法, タイムアウト秒, x, y, メッセージ=EMPTY, 表示項目, [表示項目2, ..., 表示項目29])
    :noindex:

    | セレクトボックスを表示します

    .. note:: セレクトボックスのクラス名は ``UWSCR.SlctBox`` です

    .. note:: 引数x, yについて

        | 第3、第4引数が数値であった場合はx, yが指定されたものとします
        | "100" など数値に変換できる文字列であってもここでは数値として扱われません
        | x, yの有無による表示項目として渡せる引数の数が変わります

    :param SLCT定数 表示方法: 項目の表示方法および戻り値の形式を示す定数

        | 表示方法と戻り値の形式をそれぞれ一つずつ ``OR`` で連結できます

        | 表示方法

            .. object:: SLCT_BTN

                ボタン

            .. object:: SLCT_CHK

                チェックボックス

            .. object:: SLCT_RDO

                ラジオボタン

            .. object:: SLCT_CMB

                コンボボックス

            .. object:: SLCT_LST

                リストボックス

        | 戻り値の形式

            .. object:: SLCT_STR

                項目名を返す

            .. object:: SLCT_NUM

                インデックス番号で返す

    :param 数値 タイムアウト秒: 指定秒数経過で自動的にダイアログを閉じる (キャンセル扱い)、0ならタイムアウトなし
    :param 数値 省略可 x: ダイアログの初期表示位置のX座標を指定、省略時(EMPTY)なら画面中央
    :param 数値 省略可 y: ダイアログの初期表示位置のY座標を指定、省略時(EMPTY)なら画面中央

    .. hint:: x, yに-1を指定するとそれぞれ前回表示した位置になります

    :param 文字列 省略可 メッセージ: メッセージ欄に表示されるメッセージ
    :param 文字列または配列 表示項目: 表示される項目名、または項目名を格納した配列
    :param 文字列または配列 表示項目2-31: 表示される項目名、または項目名を格納した配列
    :return:

        | ``SLCT_NUM`` および ``SLCT_STR`` 未指定時

            | 選択項目に応じた定数が返る
            | n番目の項目が選ばれれば ``SLCT_n``
            | ``SLCT_1`` から ``SLCT_31`` まで

            .. object:: SLCT_CHK, SLCT_LST 以外

                | 選択項目を示す値が返る

            .. object:: SLCT_CHK, SLCT_LST 指定時

                | 選択項目の値が合算される

                .. admonition:: 例

                    3番目と5番目が選ばれた場合 ``SLCT_3 or SLCT_5`` が返る

            .. warning:: 表示項目の配列指定で項目数が31を超える場合に、32個目以上を選択するとエラーになります

        | ``SLCT_NUM`` 指定時

            .. object:: SLCT_CHK, SLCT_LST 以外

                | 選択位置のインデックス値(0から)が返る

            .. object:: SLCT_CHK, SLCT_LST 指定時

                | 選択位置のインデックス値を格納した配列

            .. note:: 項目数が31を超えてもOK

        | ``SLCT_STR`` 指定時

            .. object:: SLCT_CHK, SLCT_LST 以外

                | 選択した項目の表示名

            .. object:: SLCT_CHK, SLCT_LST 指定時

                | 選択した項目の表示名を格納した配列

            .. note:: 項目数が31を超えてもOK

        | キャンセル時

            ``-1`` を返す

    .. admonition:: UWSCとの違い
        :class: caution

        - タイムアウト時の戻り値が0ではなく-1になった
        - 表示項目に連想配列を渡した場合、値でなはくキーが表示される
        - ``SLCT_CHK``, ``SLCT_LST`` 指定時の戻り値がタブ文字連結された文字列ではなく配列になった

.. function:: popupmenu(メニュー項目, [x=EMPTY, y=EMPTY])

    | ポップアップメニューを表示します

    :param 配列 メニュー項目: 表示項目を示す配列、要素が配列の場合サブメニューになる
    :param 数値 省略可 x: ダイアログの初期表示位置のX座標を指定、省略時(EMPTY)なら画面中央
    :param 数値 省略可 y: ダイアログの初期表示位置のY座標を指定、省略時(EMPTY)なら画面中央

    .. hint:: x, y省略時はマウスカーソル位置

    :return: 選択した項目の表示名、メニューの外側を選んだ場合はEMPTY

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // サブメニュー表示方法
            list = ["項目1", "項目2", "サブメニュー", ["サブ項目1", "サブ項目2"], "項目3"]
            // 要素を配列にすると直前の項目のサブメニューになる
            selected = popupmenu(list)
            // 項目1
            // 項目2
            // サブメニュー > サブ項目1
            //                サブ項目2
            // 項目3

            // ネストも可能
            list = ["menu", ["branch1", "branch2", ["leaf1", "leaf2"]]]
            popupmenu(list)

    .. admonition:: UWSCとの違い
        :class: caution

        - メニュー項目に連想配列を渡した場合、値ではなくキーが表示されます
        - メニュー項目を選んだ場合の戻り値が項目のインデックス値ではなく選択項目の表示名になりました
        - メニュー項目外を選んだ場合の戻り値が-1ではなくEMPTYになりました

メッセージ表示
--------------