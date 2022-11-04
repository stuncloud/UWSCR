COMオブジェクト
===============

COMオブジェクトの作成
---------------------

.. attention::

    | 以下の関数はCOM関連の実装が壊れているため現時点では利用不可となっています
    | 呼び出した場合エラーになります (**[未定義エラー] 関数が未定義です**)


.. function:: createoleobj(ProgID)

    | COMオブジェクトのインスタンスを得ます

    :param 文字列 ProgID: COMオブジェクトのProgID
    :return: :ref:`COMオブジェクト <com_object>`

.. function:: getactiveoleobj(ProgID)

    | 既に起動中のCOMオブジェクトを得ます

    :param 文字列 ProgID: COMオブジェクトのProgID
    :return: :ref:`COMオブジェクト <com_object>`


VARIANT型
---------

.. function:: vartype(値, VAR定数=-1)
    :noindex:

    | VARIANT型変数の型を調べます
    | または、指定した型へのキャストを行います

    :param VARIANT型 値: VARIANT型の値
    :param 定数 省略可 VAR定数: 型変換を行う場合に指定

        .. object:: VAR_EMPTY

            Empty

        .. object:: VAR_NULL

            Null

        .. object:: VAR_SMALLINT

            符号あり16ビット整数

        .. object:: VAR_INTEGER

            符号あり32ビット整数

        .. object:: VAR_SINGLE

            単精度浮動小数点数

        .. object:: VAR_DOUBLE

            倍精度浮動小数点数

        .. object:: VAR_CURRENCY

            通貨型

        .. object:: VAR_DATE

            日付型

        .. object:: VAR_BSTR

            BSTR型

        .. object:: VAR_DISPATCH

            COMオブジェクト

        .. object:: VAR_ERROR

            エラー値

        .. object:: VAR_BOOLEAN

            bool値

        .. object:: VAR_VARIANT

            VARIANT型

        .. object:: VAR_UNKNOWN

            IUnknown型

        .. object:: VAR_SBYTE

            符号あり8ビット整数

        .. object:: VAR_BYTE

            符号なし8ビット整数

        .. object:: VAR_WORD

            符号なし16ビット整数

        .. object:: VAR_DWORD

            符号なし32ビット整数

        .. object:: VAR_INT64

            符号あり64ビット整数

        .. object:: VAR_ARRAY

            SafeArray

        .. object:: VAR_UWSCR

            VARIANTではない値 (UWSCRの値型)

    :return:

        .. object:: VAR定数が未指定

            渡された値の型を示すVAR定数

        .. object:: VAR定数を指定した場合

            | VAR定数で指定した型にキャストされたVARIANT型の値
            | ``VAR_UWSCR`` 指定時は対応するUWSCRの値型に変換される

SafeArray
---------

.. function:: safearray([下限=0, 上限=-1, 二次元下限=EMPTY, 二次元上限=(二次元下限-1)])
.. function:: safearray(一次元配列)
    :noindex:

    | 空のSafeArrayを作成します
    | または通常の配列変数(一次元)をSafeArrayに変換します

    :param 数値 省略可 下限: SafeArrayのインデックス下限
    :param 数値 省略可 上限: SafeArrayのインデックス上限
    :param 数値 省略可 二次元下限: 二次元目のインデックス下限、省略時は二次元にしない
    :param 数値 省略可 二次元上限: 二次元目のインデックス上限
    :param 配列 一次元配列: 通常の配列 (一次元のみ)
    :return: SafeArray

    .. admonition:: UWSCとの違い
        :class: caution

        | UWSCでは下限省略時は1、上限省略時は0になっていましたが、下限は0、上限は-1となるように変更されました
        | これにより引数省略でサイズ0のSafeArrayが作成されます
        | また、引数に配列変数を渡すことで通常の配列をSafeArrayに変換できるようになりました

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            sa = safearray(0, 1) // サイズ2の配列

            sa = safearray(-2, 2) // 添字が -2,-1,0,1,2
            print length(sa) // 5

            sa = safearray() // サイズ0の配列
            // safearray(0, -1) と同等

            sa = safearray(0, 1, 0, 3)
            print length(sa, 2) // 4 (lengthの第2引数に次元数を指定できる)

            // 配列→SafeArray
            sa = safearray(["foo", "bar", "baz"])
            print sa[0] // foo