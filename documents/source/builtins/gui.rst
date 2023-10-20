GUI
===

ダイアログ
----------

.. function:: msgbox(メッセージ, [ボタン種=BTN_OK, x=EMPTY, y=EMPTY, フォーカス=EMPTY])

    | メッセージボックスを表示します

    .. admonition:: クラス名
        :class: hint

        | メッセージボックスのクラス名は ``UWSCR.MsgBox`` です

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

        .. admonition:: 前回表示位置に表示
            :class: hint

            | x, yに-1を指定するとそれぞれ前回表示した位置になります

    :param ボタン定数 省略可 フォーカス: カーソルの初期位置をボタン定数で指定、省略時や該当ボタンがない場合は一番左のボタンがフォーカスされます

    :return: 押されたボタンを示すボタン定数 (×ボタンで閉じられた場合は ``BTN_CANCEL``)

.. function:: input(メッセージ, [デフォルト値=EMPTY, マスク表示=FALSE, x=EMPTY, y=EMPTY])

    | インプットボックスを表示します

    .. admonition:: クラス名
        :class: hint

        | インプットボックスのクラス名は ``UWSCR.Input`` です

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

        .. admonition:: 前回表示位置に表示
            :class: hint

            | x, yに-1を指定するとそれぞれ前回表示した位置になります

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

    .. admonition:: クラス名
        :class: hint

        | セレクトボックスのクラス名は ``UWSCR.SlctBox`` です

    .. admonition:: 引数x, yについて
        :class: note

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

        .. admonition:: 前回表示位置に表示
            :class: hint

            | x, yに-1を指定するとそれぞれ前回表示した位置になります

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
    :param 数値 省略可 x: メニュー表示位置のX座標を指定、省略時(EMPTY)はマウスカーソル位置
    :param 数値 省略可 y: メニュー表示位置のY座標を指定、省略時(EMPTY)はマウスカーソル位置

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

.. function:: balloon(メッセージ, [X=0, Y=0, 向き=0, フォントサイズ=既定値, 文字色=$000000, 背景色=$00FFFF, 透過=0])
.. function:: fukidasi(メッセージ, [X=0, Y=0, 向き=0, フォントサイズ=既定値, 文字色=$000000, 背景色=$00FFFF, 透過=0])

    | 吹き出しを表示します

    :param 文字列 メッセージ: 表示するメッセージ
    :param 数値 省略可 X: 表示位置 (X座標)
    :param 数値 省略可 Y: 表示位置 (Y座標)
    :param 数値 省略可 向き: **未実装**
    :param 数値 省略可 フォントサイズ: 表示される文字のサイズ、省略時はフォント設定に従う
    :param 数値 省略可 フォント名: 表示される文字のフォント、省略時はフォント設定に従う
    :param 数値 省略可 文字色: 文字の色をBGR値で指定、省略時は黒
    :param 数値 省略可 背景色: 背景の色をBGR値で指定、省略時は黄色

        .. hint:: BGRの例

                - 青: ``$FF0000``
                - 緑: ``$00FF00``
                - 赤: ``$0000FF``
                - 白: ``$FFFFFF``
                - 黒: ``$000000``
                - 黄: ``$00FFFF``

        .. admonition:: UWSCとの違い
            :class: hint

            | 色指定を0にした場合、黄色ではなく黒になります

    :param 数値 省略可 透過: **未実装**
    :return: なし

    .. hint:: スレッド毎に一つの吹き出しを表示できます


.. function:: logprint(表示フラグ, [X=EMPTY, Y=EMPTY, 幅=EMPTY, 高さ=EMPTY])

    | printウィンドウの表示状態を変更します

    :param 真偽値 表示フラグ:

        .. object:: TRUE

            | print窓を表示する

        .. object:: FALSE

            | print窓を非表示にする
            | 既に表示済みなら消す

    :param 数値 省略可 X: 表示位置 (X座標)、EMPTYなら現状維持
    :param 数値 省略可 Y: 表示位置 (Y座標)、EMPTYなら現状維持
    :param 数値 省略可 幅: 表示サイズ (幅)、EMPTYなら現状維持
    :param 数値 省略可 高さ: 表示サイズ (高さ)、EMPTYなら現状維持
    :return: なし

HTMLフォーム
------------

.. function:: createform(HTMLファイル, タイトル, [非同期フラグ=FALSE, オプション=FOM_DEFAULT, 幅=EMPTY, 高さ=EMPTY, X=EMPTY, Y=EMPTY])

    | 関数の説明

    .. admonition:: WebView2 Runtimeが必要です
        :class: caution

        | ``Microsoft Edge WebView2 Runtime`` がインストールされていない場合この関数はエラーになります

    .. admonition:: UWSCとは互換性がありません
        :class: warning

        | UWSCではIEコンポーネントを利用していたのに対してUWSCRではWebView2を利用しています
        | そのためUWSCで実行していたコードが動作しない場合があります

    :param 文字列 HTMLファイル: 表示したいHTMLファイルのパス

        .. admonition:: ファイルの配置について
            :class: hint

            | HTMLファイルから別のファイルを参照する場合、もとのHTMLファイルを起点とした相対パスを指定します
            |

            - C:\\Test\\
                - form.html
                    - js\\
                        - form.js
                    - css\\
                        - form.css
                    - img\\
                        - form.png

            .. code-block:: html

                <!DOCTYPE html>
                <html lang="ja">
                <head>
                    <meta charset="UTF-8">
                    <title>別ファイル参照例</title>
                    <link rel="stylesheet" href="css/form.css">
                    <script src="js/form.js"></script>
                </head>
                <body>
                    <img src="img/form.png">
                    <form>
                        <input type="submit" value="OK" name="OK">
                    </form>
                </body>
                </html>

            .. sourcecode:: uwscr

                html = "c:\test\form.html"
                r = createform(html, "test")

    :param 文字列 タイトル: ウィンドウタイトル
    :param 真偽値 省略可 非同期フラグ: 非同期で実行するかどうか

        - FALSE: submitボタンが押される、またはウィンドウが閉じられるまで待機する
        - TRUE: 関数実行後にウィンドウが表示されたら制御を返す

    :param 定数 省略可 オプション: 以下の定数の組み合わせ(OR連結)を指定

        .. object:: FOM_NOICON

            | 閉じるボタンを非表示にする

        .. object:: FOM_MINIMIZE

            | 最小化ボタンを表示する

        .. object:: FOM_MAXIMIZE

            | 最大化ボタンを表示する

        .. object:: FOM_NOHIDE

            | submitボタンが押されてもウィンドウを閉じない

        .. object:: FOM_NOSUBMIT

            | submitボタンが押されてもsubmitに割り当てられた処理(action)を行わない

        .. object:: FOM_NORESIZE

            | ウィンドウのサイズ変更不可

        .. object:: FOM_BROWSER

            | 互換性のために残されていますが使用できません (指定しても無視されます)

        .. object:: FOM_FORMHIDE

            | ウィンドウを非表示で起動する

        .. object:: FOM_TOPMOST

            | ウィンドウを最前面に固定

        .. object:: FOM_NOTASKBAR

            | タスクバーにアイコンを表示しない

        .. object:: FOM_FORM2

            | 互換性のために残されていますが使用できません (指定しても無視されます)

        .. object:: FOM_DEFAULT

            | オプションなし (0)

    :param 数値 省略可 幅: ウィンドウの幅
    :param 数値 省略可 高さ: ウィンドウの高さ
    :param 数値 省略可 X: ウィンドウのX座標
    :param 数値 省略可 Y: ウィンドウのY座標
    :rtype: :ref:`form_data` または :ref:`form_object`
    :return: 非同期フラグによる

        - FALSE: :ref:`form_data`
        - TRUE: :ref:`form_object`

    .. code-block:: html

        <!DOCTYPE html>
        <html lang="ja">
        <head>
            <meta charset="UTF-8">
            <title>Sample.html</title>
        </head>
        <body>
            <form>
                <div>
                    <span>ユーザー名</span>
                    <input type="text" name="user">
                </div>
                <div>
                    <span>パスワード</span>
                    <input type="password" name="pwd">
                </div>
                <div>
                    <select name="slct">
                        <option value="foo">foo</option>
                        <option value="bar">bar</option>
                        <option value="baz">baz</option>
                    </select>
                </div>
                <div>
                    <textarea name="txt" cols="30" rows="10"></textarea>
                </div>
                <div>
                    <input type="submit" value="OK" name="OK">
                    <input type="submit" value="Cancel" name="Cancel">
                </div>
            </form>
        </body>
        </html>

    .. sourcecode:: uwscr

        r = createform("sample.html", "Sample")
        select r.submit
            case "OK"
                print "OKが押されました"
                print "formの値は以下です"
                for data in r.data
                    print data.name + ": " + data.value
                next
            case "Cancel"
                print 'キャンセルされました'
            case NULL
                print 'submitされずにウィンドウが閉じられました'
        selend

.. _form_data:

Form情報
^^^^^^^^

submit時のform情報を示す :ref:`uobject`

.. code-block:: js

    // submit時
    {
        "submit": $submit, // $submitには押されたsubmitボタンのnameが入る
        "data": [
            // form内の各要素のnameおよびvalueが格納される
            { "name": $name, "value", $value},
        ]
    }
    // ウィンドウが閉じられた場合
    {
        "submit": null, // NULLになる
        "data": []      // 空配列
    }

.. _form_object:

Formオブジェクト
^^^^^^^^^^^^^^^^

| Formウィンドウを示すオブジェクト

.. admonition:: COMオブジェクトではありません
    :class: caution

    | UWSCとは異なりCOMオブジェクトではなくUWSCR独自のオブジェクトとなります

.. class:: Form

    .. property:: Document

        | フォームに表示されているページのdocumentオブジェクト

        :rtype: :ref:`webview_remote_object`

    .. method:: Wait()

        | ウィンドウが閉じられるのを待つ

        :rtype: :ref:`form_data`
        :return:

            | submit時のform情報を示す :ref:`form_data` オブジェクト
            | submitせず閉じた場合は ``submit`` がNULLになります

        .. sourcecode:: uwscr

            // test.htmlにはOKとCancelのsubmitボタンがあるものとする
            f = createform("test.html", "Test", true)
            result = f.wait()
            select result.submit
                case "OK"
                    for data in result.data
                        print data.name + ": " + data.value
                    next
                case "Cancel"
                    print "キャンセルされました"
                case NULL
                    print "ウィンドウが閉じられました"
                default
                    print "なにかおかしいです"
            selend

    .. method:: SetVisible([表示フラグ=TRUE])

        | ウィンドウの表示状態を変更する

        :param 真偽値 省略可 表示フラグ: TRUEで表示、FALSEで非表示
        :return: なし

    .. method:: Close()

        | ウィンドウを閉じる

        :return: なし

    .. method:: SetEventHandler(エレメント, イベント, 関数)

        | 任意のイベント発生時に実行する関数を登録します
        | 関数は引数を2つまで受けられます、内訳は以下の通りです

        1. イベント発生エレメントのvalue値
        2. イベント発生エレメントのname属性値

        :param WebViewRemoteObject エレメント: イベント発生元のエレメントを示す :ref:`webview_remote_object`
        :param 文字列 イベント: イベント名
        :param ユーザー定義関数 関数: イベント発生時に実行される関数
        :return: なし

        .. sourcecode:: uwscr

            f = createform("test.html", "Test", true)
            select = f.document.querySelector("select")
            f.SetEventHandler(select, "change", on_select_change)

            button = f.document.querySelector("input[type=button]")
            f.SetEventHandler(button, "click", on_button_click)

            f.wait()

            // 1つ目の引数でイベント発生エレメントのvalue
            // 2つ目の引数でnameを受ける
            procedure on_select_change(value, name)
                print value
                print name
            fend

            // 引数は必須ではない
            procedure on_button_click()
                print "クリックされました"
            fend

.. _webview_remote_object:

WebViewRemoteObject
^^^^^^^^^^^^^^^^^^^

| フォームに表示されているページのJavaScriptオブジェクトを示します
| 利用方法は :ref:`remote_object` と同等です

