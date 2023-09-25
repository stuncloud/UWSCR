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

VARIANT
-------

.. function:: vartype(値, VAR定数=EMPTY)

    | Variant型のデータ型をVAR定数で返します
    | または指定した型のVariant型に変換します

    :param 全て 値: 任意の値
    :param 定数 省略可 VAR定数: Variant型を得る場合に指定

        .. admonition:: 以下の定数は使用できません
            :class: caution

            - ``VAR_ASTR``
            - ``VAR_USTR``
            - ``VAR_UWSCR``

    :rtype: VAR定数、またはVariant型
    :return:

        - VAR定数指定時: 変換されたVariant型
        - VAR定数未指定時: 渡された値の型を示すVAR定数

    .. sourcecode:: uwscr

        // 開いているExcelを取得
        excel = getactiveoleobj("Excel.Application")
        // 日付型のVariantに変換
        date = vartype("2023/07/15", VAR_DATE)
        // Excelのアクティブセルに日付型の値を入力
        excel.ActiveCell.value = date

.. function:: vartype(COMオブジェクト, プロパティ名)

    | COMオブジェクトのプロパティが返す値のVariant型を得ます

    :param COMオブジェクト COMオブジェクト: 型を調べたいプロパティを持つCOMオブジェクト
    :param 文字列 プロパティ名: 型を調べたいプロパティの名前
    :rtype: VAR定数またはEMPTY
    :return: プロパティの型、COMオブジェクト以外が渡された場合やプロパティが存在しない場合はEMPTY

    .. sourcecode:: uwscr

        excel = getactiveoleobj("Excel.Application")
        // アクティブセルの型を調べる
        vt = vartype(excel.activecell, "value")
        // 得た値をVAR_定数名に変換
        print const_as_string(vt, "VAR_")

VAR定数
^^^^^^^

.. list-table:: VAR定数一覧
    :header-rows: 1

    * - 定数
      - 値
      - 詳細
    * - VAR_EMPTY
      - 0
      - EMPTY
    * - VAR_NULL
      - 1
      - NULL
    * - VAR_SMALLINT
      - 2
      - 符号付き2バイト整数
    * - VAR_INTEGER
      - 3
      - 符号付き4バイト整数
    * - VAR_SINGLE
      - 4
      - 単精度浮動小数点数
    * - VAR_DOUBLE
      - 5
      - 倍精度浮動小数点数
    * - VAR_CURRENCY
      - 6
      - 通貨型
    * - VAR_DATE
      - 7
      - 日付型
    * - VAR_BSTR
      - 8
      - 文字列型
    * - VAR_DISPATCH
      - 9
      - IDispatch型 (COMオブジェクト)
    * - VAR_ERROR
      - 10
      - エラー
    * - VAR_BOOLEAN
      - 11
      - 真偽値
    * - VAR_VARIANT
      - 12
      - VARIANT型
    * - VAR_UNKNOWN
      - 13
      - IUnknown型
    * - VAR_SBYTE
      - 16
      - 符号付き1バイト整数
    * - VAR_BYTE
      - 17
      - 符号なし1バイト整数
    * - VAR_WORD
      - 18
      - 符号なし2バイト整数
    * - VAR_DWORD
      - 19
      - 符号なし4バイト整数
    * - VAR_INT64
      - 20
      - 符号付き8バイト整数
    * - VAR_ASTR
      - 256
      - 互換性のために残していますが実際には使用できません
    * - VAR_USTR
      - 258
      - 互換性のために残していますが実際には使用できません
    * - VAR_UWSCR
      - 512
      - UWSCRのデータ型
    * - VAR_ARRAY
      - $2000 (8192)
      - 配列




非推奨関数
----------

.. admonition:: 非推奨の理由
    :class: caution

    | UWSCRにはSAFEARRAY型の値が存在しないため以下の関数は非推奨となりました
    | 互換性のため関数は残していますが、UWSCとは結果が異なります

.. function:: safearray([下限=0, 上限=-1, 二次元下限=EMPTY, 二次元上限=(二次元下限-1)])

    | EMPTYを返します

    :param 数値 省略可 下限: 無視されます
    :param 数値 省略可 上限: 無視されます
    :param 数値 省略可 二次元下限: 無視されます
    :param 数値 省略可 二次元上限: 無視されます
    :return: EMPTY

