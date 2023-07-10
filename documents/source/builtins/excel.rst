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

        | ファイルオープン時の追加パラメータを ``"パラメータ名:=値"`` 形式の文字列で指定する
        | 書式が不正な場合は無視される (エラーにはなりません)
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

.. function:: xlclose(Excel, [ファイル名=FALSE])

    | Excelを終了します
    | ファイル名指定の有無で保存方法が異なります

    :param COMオブジェクト Excel: Excel.ApplicationまたはWorkbookを示すCOMオブジェクト
    :param 文字列 省略可 ファイル名: 保存するファイル名を指定、省略時は上書き保存

.. function:: xlclose(Excel, TRUE)
    :noindex:

    | 変更内容を保存せずに終了します

    :param COMオブジェクト Excel: Excel.ApplicationまたはWorkbookを示すCOMオブジェクト
    :param 真偽値 TRUE: ``TRUE`` を指定 (固定値)
    :rtype: 真偽値
    :return: 成功時TRUE、失敗時FALSE

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            excel = xlopen("foo.xlsx")
            // ブックが編集される
            xlclose(excel, "bar.xlsx") // 別名で保存

            excel = xlopen("bar.xlsx")
            // ブックが編集される
            xlclose(excel) // 上書き保存

            excel = xlopen("foo.xlsx")
            // ブックが編集される
            xlclose(excel, TRUE) // 保存せず終了

.. function:: xlactivate(Excel, シート識別子, [ブック識別子=EMPTY])

    | 指定したシートをアクティブにします

    :param COMオブジェクト Excel: Excel.ApplicationまたはWorkbookを示すCOMオブジェクト
    :param 文字列または数値 シート識別子: アクティブにするシート名またはインデックス番号(1から)
    :param 文字列または数値 省略可 ブック識別子: アクティブにするブック名またはインデックス番号(1から)
    :rtype: 真偽値
    :return: 成功時TRUE、失敗時FALSE

    .. admonition:: シート・ブックの識別子について
        :class: hint

        - シート名は各シートの表示名を完全一致で指定する必要があります
        - シートのインデックス番号は左から数えた順番です
        - ブック名はファイル名を完全一致で指定する必要があります
            - 新規作成したブックの場合は ``Book1`` のようになります
        - ブックのインデックス番号はブックを開いた順番です
        - ブック識別子を省略した場合はアクティブなブックが対象となります
        - Workbookオブジェクトを指定した場合ブック識別子は無視され、そのWorkbook内のシートをアクティブにします

.. function:: xlsheet(Excel, シート識別子, [削除=FALSE])

    | アクティブなブックへのシートの追加、または削除を行う

    :param COMオブジェクト Excel: Excel.ApplicationまたはWorkbookを示すCOMオブジェクト
    :param 文字列または数値 シート識別子: アクティブにするシート名、削除時のみインデックス番号(1から)も可
    :param 真偽値 省略可 削除: FALSEなら指定名のシートを追加、TRUEなら該当シートを削除
    :rtype: 真偽値
    :return: 成功時TRUE、失敗時FALSE

        .. admonition:: インデックス指定について
            :class: hint

            | シート追加時はインデックス番号を文字列として扱います

            .. sourcecode:: uwscr

                xlsheet(excel, 1, FALSE) // "1" という名前のシートが追加される

            | シート削除時はインデックスとシート名を厳密に区別します
            | そのためUWSCとは一部動作が異なります

            .. sourcecode:: uwscr

                xlsheet(excel, 1, FALSE) // "1" という名前のシートを追加しておく
                xlsheet(excel, 1, TRUE)  // 1を指定して削除を試みた場合
                // UWSCの場合: "1" という名前のシートがあればそれを削除、なければ1番目のシートを削除
                // UWSCRの場合: 必ず1番目のシートを削除、2番目以降にある"1"という名前のシートは対象とならない
