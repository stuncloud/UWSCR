Excel
=====

.. function:: xlopen([ファイル名=EMPTY, 起動フラグ=XL_DEFAULT, パラメータ...])

    | Excelを起動します

    :param 文字列 省略可 ファイル名: 開きたいファイル名、EMPTYならExcelを新規に起動
    :param 定数 省略可 起動フラグ: Excelの起動方法を指定

        .. object:: XL_DEFAULT (0)

            | 起動済みのExcelがあればそれを使い、なければ新規起動します

        .. object:: XL_NEW (1)

            | 常にExcelを新規に起動します

        .. object:: XL_BOOK (2)

            | applicationではなくWorkbookオブジェクトを返します

        .. object:: XL_OOOC (3)

            | 使用できません

    :param 文字列 可変長 パラメータ:

        | ファイルオープン時の追加パラメータを ``"パラメータ名:=値"`` で指定する
        | 書式が不正な場合は無視される
        | 以下は有効なパラメータ例

        .. object:: UpdateLinks

            | リンク更新方法

            - 0: 更新しない
            - 1: 外部更新のみ
            - 2: リモート更新のみ
            - 3: 外部、リモート共に更新

        .. object:: ReadOnly

            | 読み取り専用で開く場合にTrueを指定

        .. object:: Format

            | CSVファイル時を開く場合にその区切り文字

            - 1: タブ
            - 2: カンマ
            - 3: スペース
            - 4: セミコロン

        .. object:: Password

            | パスワード保護されたブックを開くためのパスワード

        .. object:: WriteResPassword

            | 書き込み保護されたブックに書き込むためのパスワード

        .. object:: IgnoreReadOnly

            | 「読み取り専用を推奨する」のダイアログを抑止したい場合にTrue


        .. sourcecode:: uwscr

            // パスワード付きファイルを読み取り専用で開く
            excel = xlopen("hoge.xlsx", XL_NEW, "ReadOnly:=True", "Password:=hogehoge")

            // カンマ区切りcsvファイルを開く
            excel = xlopen("hoge.xlsx", XL_NEW, "Format:=2")

    :rtype: :ref:`com_object`
    :return: Excel自身、またはWorkbookを示すCOMオブジェクト
