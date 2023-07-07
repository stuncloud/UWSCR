COMオブジェクト
===============

COMオブジェクトの作成・取得
---------------------------

.. function:: createoleobj(ProgID)

    | COMオブジェクトのインスタンスを得ます

    :param 文字列 ProgID: COMオブジェクトのProgIDまたはCLSID
    :return: :ref:`COMオブジェクト <com_object>`

.. function:: getactiveoleobj(ProgID, [タイトル=EMPTY, n番目=1])

    | 既に起動中のCOMオブジェクトを得ます
    | タイトルが未指定の場合は指定ProgIDに該当しアクティブなオブジェクトを返します
    | タイトルを指定した場合はウィンドウタイトルに部分一致するウィンドウからProgIDに該当するオブジェクトを返します

    :param 文字列 ProgID: COMオブジェクトのProgIDまたはCLSID
    :param 文字列 省略可 タイトル: ExcelやWordなど、オブジェクトを取得したいウィンドウのタイトルを指定 (部分一致)

        .. admonition:: MDI非対応
            :class: caution

            | MDIウィンドウは対象外です

    :param 数値 省略可 n番目: タイトルに一致するウィンドウが複数ある場合、n番目を取得
    :return: :ref:`COMオブジェクト <com_object>`

.. admonition:: CLSIDの入力
    :class: hint

    | CLSIDは ``{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}`` の形式で入力します

    .. sourcecode:: uwscr

        // WScript.ShellのCLSIDを指定
        ws = createoleobj("{72C24DD5-D70A-438B-8A42-98424B88AFB8}")
        print ws // ComObject(IWshShell3)
        ws.Popup("Hello!")

コレクション
------------

.. function:: getoleitem(コレクション)

    | コレクションを配列に変換します

    :param COMオブジェクト コレクション: コレクションを示すCOMオブジェクト
    :rtype: 配列
    :return: コレクションの要素を格納した配列

    .. admonition:: UWSCとの違い
        :class: caution

        | 要素の数ではなく要素の配列を返すようになりました
        | それに伴い ``ALL_OLE_ITEM`` は廃止されました

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            ws = createoleobj("WScript.Shell")
            col = getoleitem(ws.SpecialFolders)
            print col[0]

.. イベント
.. --------

.. .. function:: oleevent(COMオブジェクト, インターフェース名, イベント名, 関数)

..     | COMオブジェクトのイベント発生時に実行するユーザー定義関数(イベントハンドラ)を指定します

..     :param COMオブジェクト COMオブジェクト: イベントハンドラをセットする :ref:`com_object`
..     :param 文字列 インターフェース名: イベントを実装するインターフェース名
..     :param 文字列 イベント名: フックするイベントの名前
..     :param 関数または文字列 関数: ユーザー定義関数またはその関数名
..     :return: なし

.. .. function:: oleevent(COMオブジェクト)
..     :noindex*

..     | COMオブジェクトにセットされた全てのイベントハンドラを解除します

..     :param COMオブジェクト COMオブジェクト: イベントを解除したい :ref:`com_object`
..     :return: なし

非推奨関数
----------

.. admonition:: 非推奨の理由
    :class: caution

    | UWSCRではVARIANT型及びSAFEARRAY型の値が存在しないため以下の関数は非推奨となりました
    | 互換性のため関数は残していますが、UWSCとは結果が異なります

.. function:: vartype(値, VAR定数=-1)
    :noindex:

    | ``VAR_UWSCR`` を返します

    :param 全て 値: 値
    :param 定数 省略可 VAR定数: 無視されます
    :return: ``VAR_UWSCR``

.. function:: safearray([下限=0, 上限=-1, 二次元下限=EMPTY, 二次元上限=(二次元下限-1)])

    | EMPTYを返します

    :param 数値 省略可 下限: 無視されます
    :param 数値 省略可 上限: 無視されます
    :param 数値 省略可 二次元下限: 無視されます
    :param 数値 省略可 二次元上限: 無視されます
    :return: EMPTY

