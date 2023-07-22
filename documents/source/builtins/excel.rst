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

.. function:: xlclose(Excel, [ファイル名])

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
        - ブック名は拡張子を含めたファイル名を完全一致で指定する必要があります
            - 新規作成したブックの場合は ``Book1`` のようになります(拡張子がありません)
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

.. function:: xlgetdata(Excel, [範囲=EMPTY, シート識別子=EMPTY])
.. function:: xlgetdata(Excel, [範囲=EMPTY, <EMPTYPARAM>, シート識別子=EMPTY])
    :noindex:

    | 範囲をA1形式の文字列で指定し、その値を返します

    :param COMオブジェクト Excel: Excel.ApplicationまたはWorkbookを示すCOMオブジェクト
    :param 文字列 省略可 範囲: 単一セル指定なら"A1"、範囲なら"A1:C3"のように指定
    :param 文字列または数値 省略可 シート識別子: 得たい値のあるシート名またはインデックス番号(1から)を指定、省略時はアクティブシート

        .. admonition:: 第三引数について
            :class: hint

            | 互換性のために第三引数を省略し、第四引数にシート名を指定することもできます

    :rtype: 値または配列、値の型はセルによる
    :return: 範囲の指定方法により異なります

        - 単一セル指定: セルの値を返す
        - 範囲指定: 範囲内の値を順に格納した配列を返す

        .. admonition:: 範囲指定時の注意
            :class: caution

            | UWSCではインデックスが1から始まるSafeArrayが返っていましたが
            | UWSCRでは通常の配列が返るためインデックスが0からになります

.. function:: xlgetdata(Excel, 行番号, 列番号, [シート識別子=EMPTY])
    :noindex:

    | セルの行と列の番号を指定しその値を得ます

    :param COMオブジェクト Excel: Excel.ApplicationまたはWorkbookを示すCOMオブジェクト
    :param 数値 行番号: 値を得たいセルの行番号 (1から)
    :param 数値 列番号: 値を得たいセルの列番号 (1から)
    :param 文字列または数値 省略可 シート識別子: 得たい値のあるシート名またはインデックス番号(1から)を指定、省略時はアクティブシート
    :rtype: セルによる
    :return: 指定セルの値

.. function:: xlsetdata(Excel, 値, [範囲=EMPTY, シート識別子=EMPTY, 文字色=EMPTY, 背景色=EMPTY])
.. function:: xlsetdata(Excel, 値, [範囲=EMPTY, <EMPTYPARAM>, シート識別子=EMPTY, 文字色=EMPTY, 背景色=EMPTY])
    :noindex:

    | A1形式で指定したセルまたはセル範囲に値を入力します
    | 入力したい値が配列で、かつ単一セルを指定した場合は指定セルを起点として配列の値を入力します

    :param COMオブジェクト Excel: Excel.ApplicationまたはWorkbookを示すCOMオブジェクト
    :param 値 値: 入力したい値 (配列可)

        .. admonition:: 入力値ごとの入力パターン
            :class: hint

            入力値により入力方法が代わります

            - 単一の値: 指定範囲すべてに同一の値が入力されます
            - 一次元配列: 指定行の列ごとに配列要素がそれぞれ入力されます、範囲が複数行の場合それぞれの行に入力されます
            - 二次元配列: 配列を行列とみなし各要素を該当するセルに入力します

            | 配列サイズが指定範囲を超える場合、超過分は入力されません
            | 指定範囲が配列サイズを超える場合、不足箇所には ``#N/A`` が入力されます

    :param 文字列 省略可 範囲: A1形式でセルまたはセル範囲を指定
    :param 文字列または数値 省略可 シート識別子: 得たい値のあるシート名またはインデックス番号(1から)を指定、省略時はアクティブシート

        .. admonition:: 第三引数について
            :class: hint

            | 互換性のために第三引数を省略し、第四引数にシート名を指定することもできます

    :param 数値 省略可 文字色: 該当セルの文字色を変更する場合にBGRで指定
    :param 数値 省略可 背景色: 該当セルの背景色を変更する場合にBGRで指定
    :rtype: 真偽値
    :return: 成功時TRUE、失敗時FALSE

.. function:: xlsetdata(Excel, 値, 行, 列, [シート識別子=EMPTY, 文字色=EMPTY, 背景色=EMPTY])
    :noindex:

    | 行列番号で指定したセルに値をセットする
    | 入力したい値が配列の場合は指定セルを起点に配列の値を入力します

    :param COMオブジェクト Excel: Excel.ApplicationまたはWorkbookを示すCOMオブジェクト
    :param 値 値: 入力したい値 (配列可)
    :param 数値 行: 入力したいセルの行番号 (1から)
    :param 数値 列: 入力したいセルの列番号 (1から)
    :param 文字列または数値 省略可 シート識別子: 得たい値のあるシート名またはインデックス番号(1から)を指定、省略時はアクティブシート
    :param 数値 省略可 文字色: 該当セルの文字色を変更する場合にBGRで指定
    :param 数値 省略可 背景色: 該当セルの背景色を変更する場合にBGRで指定
    :rtype: 真偽値
    :return: 成功時TRUE、失敗時FALSE

    .. sourcecode:: uwscr

        // A1セルに100が入力される
        xlsetdata(excel, 100, "A1")

        // A2,B2,C2に200が入力される
        xlsetdata(excel, 200, "A2:C2")

        // A3に301, B3に302, C3に303が入力される
        xlsetdata(excel, [301,302,303], "A3:C3")

        // 単一セル指定で配列を渡した場合はそのセルを起点に配列の値が入力される
        // A4に401, B4に402, C4に403になる
        xlsetdata(excel, [401,402,403], "A4")

        // 配列サイズが範囲より大きい場合
        // C5に503は入力されない
        xlsetdata(excel, [501,502,503], "A5:B5")

        // 配列サイズが範囲より小さい場合
        // 配列を超えた部分であるD6は#N/A
        xlsetdata(excel, [601,602,603], "A6:D6")

        // 二次元配列は行列になる
        //   |  A  |  B  |
        // 7 |  701|  702|
        // 8 |  801|  802|
        xlsetdata(excel, [[701,702],[801,802]], "A7:B8")

        // 二次元配列で単一セルを指定した場合でもそのセルを起点に入力される
        xlsetdata(excel, [[901,902,903],[1001,1002,1003]], "A9")

        // 行列番号指定
        xlsetdata(excel, [[1101,1102],[1201,1202],[1301,1302]], 11, 1)
