配列操作関数
============

配列の変更
----------

.. function:: qsort(var キー配列, [ソート順=QSRT_A, var 連動配列, ...])

    | 配列内の要素を並び替えます

    .. admonition:: ソート時の値型について
        :class: note

        | それぞれの値を文字列として扱いソートを行います

    :param 配列 参照渡し キー配列: ソートする配列
    :param 定数 省略可 ソート順: ソート順を示す定数

        .. object:: QSRT_A

            昇順

        .. object:: QSRT_D

            降順

        .. object:: QSRT_UNICODEA

            UNICODE文字列順 昇順

        .. object:: QSRT_UNICODED

            UNICODE文字列順 降順

        .. object:: QSRT_NATURALA

            数値順 昇順

        .. object:: QSRT_NATURALD

            数値順 降順

    :param 配列 省略可 参照渡し 連動配列: キー配列のソートに連動してソートされる配列

        | キー配列よりサイズの小さい配列はソート前にリサイズされEMPTYで埋められます
        | 8つまで指定可能

    :return: なし

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // 連動ソート
            // キー配列を並び替え、それと同じように別の配列も並び替えます
            key  = [5,2,1,4,3]
            arr1 = ["お","い","あ","え","う"]
            arr2 = ["お","い","あ","え","う", "か"] // 余分はソート対象外、この場合「か」は位置が変更されない
            arr3 = ["お","い","あ","え"] // 不足の場合末尾にEMPTYが追加されてからソート

            qsort(key, QSRT_A, arr1, arr2, arr3)
            print key  // [1, 2, 3, 4, 5]
            print arr1 // [あ, い, う, え, お]
            print arr2 // [あ, い, う, え, お, か]
            print arr3 // [あ, い, , え, お]

            qsort(key, QSRT_D, arr1, arr2, arr3)
            print key  // [5, 4, 3, 2, 1]
            print arr1 // [お, え, う, い, あ]
            print arr2 // [お, え, う, い, あ, か]
            print arr3 // [お, え, , い, あ]

.. function:: reverse(var 配列)

    | 配列の順序を反転させます

    :param 配列 参照渡し 配列: 順序を反転させたい配列
    :return: なし

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            arr = [1,2,3]
            print arr // [1,2,3]
            reverse(arr)
            print arr // [3,2,1]

.. function:: resize(var 配列, [インデックス値=EMPTY, 初期値=EMPTY])

    | 配列サイズを変更します

    :param 配列 参照渡し 配列: サイズを変更したい配列
    :param 数値 省略可 インデックス値:

            | 指定値 + 1 のサイズに変更される
            | 省略時は変更なし
            | マイナス指定時はサイズ0の配列になる

    :param 値 省略可 初期値: 元のサイズより大きくなる場合、追加される要素の初期値
    :return: 配列サイズ - 1 (配列インデックスの最大値)

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            arr = [1,2,3]
            // サイズ指定なしの場合は配列に変更なし
            print resize(arr) // 2
            print length(arr) // 3

            // サイズ指定
            print resize(arr, 3) // 3
            print length(arr) // 4

            // マイナス指定でサイズ0になる
            print resize(arr, -1) // -1
            print length(arr) // 0

            // サイズ変更+初期値指定
            arr = []
            print resize(arr, 2, "a") // 2
            print length(arr) // 3
            print arr // [a, a, a]

.. function:: setclear(var 配列, [値=EMPTY])

    | 指定した値で配列を埋めます

    :param 配列 参照渡し 配列: 値を埋めたい配列
    :param 値 省略可 値: 埋める値
    :return: なし

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            arr = [1,2,3,4,5]
            print arr // [1, 2, 3, 4, 5]

            // 値省略時はEMPTYで埋められる
            setclear(arr)
            print arr // [, , , , ]

            setclear(arr, 111)
            print arr // [111, 111, 111, 111, 111]

.. function:: shiftarray(var 配列, シフト値)

    | 指定値分配列内の要素をずらします

    :param 配列 参照渡し 配列: 対象の配列
    :param 数値 シフト値: 正の数なら要素を後方にずらす、負の数なら前方へずらす (空いた場所はEMPTYで埋められる)
    :return: なし

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            arr = [1,2,3,4,5]
            print arr // [1, 2, 3, 4, 5]
            shiftarray(arr, 2)
            print arr // [, , 1, 2, 3]
            shiftarray(arr, -2)
            print arr // [1, 2, 3, , ]

配列長を得る
------------

.. function:: Length

    | 文字列操作関数の :any:`length` 関数を参照

配列要素を使う
--------------

.. function:: slice(配列, [開始=0, 終了=EMPTY])

    | 配列の一部をコピーし新たな配列を得ます

    :param 配列 配列: コピー元の配列
    :param 数値 省略可 開始: コピーする開始位置のインデックス値
    :param 数値 省略可 終了: コピーする終了位置のインデックス値、省略時は最後まで
    :return: コピーされた配列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            // 開始と終了が未指定の場合は配列がそのまま複製される
            base = [1,2,3,4,5]
            new = slice(base)
            print new // [1, 2, 3, 4, 5]

            print slice(base, 2) // [3, 4, 5]
            print slice(base, , 2) // [1, 2, 3]
            print slice(base, 1, 3) // [2, 3, 4]

            // 範囲外が指定されたら空配列が返る
            print slice(base, 5) // []

.. function:: calcarray(配列, 計算方法, [開始=0, 終了=EMPTY])

    | 配列内の数値で計算を行います

    :param 配列 配列: 数値を含む配列 (数値以外は無視される)
    :param 定数 計算方法: 計算方法を示す定数

        .. object:: CALC_ADD

            合計値を得る

        .. object:: CALC_MIN

            最小値を得る

        .. object:: CALC_MAX

            最大値を得る

        .. object:: CALC_AVR

            平均値を得る

    :return: 計算結果

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            arr = [1,2,3,4,5]

            print calcarray(arr, CALC_ADD) // 15
            print calcarray(arr, CALC_MIN) // 1
            print calcarray(arr, CALC_MAX) // 5
            print calcarray(arr, CALC_AVR) // 3

            // 範囲指定
            print calcarray(arr, CALC_ADD, 2, 3) // 7
            print calcarray(arr, CALC_MIN, 2, 3) // 3
            print calcarray(arr, CALC_MAX, 2, 3) // 4
            print calcarray(arr, CALC_AVR, 2, 3) // 3.5

            // 数値以外は無視される
            arr = [1,2,"foo",4,5]
            print calcarray(arr, CALC_ADD) // 12
            print calcarray(arr, CALC_MIN) // 1
            print calcarray(arr, CALC_MAX) // 5
            print calcarray(arr, CALC_AVR) // 3 ※ 数値要素が4つなので (1+2+4+5) / 4


文字列との相互変換
------------------

.. function:: join(配列, [区切り文字=" ", 空文字除外=FALSE, 開始=0, 終了=(配列長-1)])

    | 配列要素を区切り文字で結合します

    :param 配列 配列: 結合したい配列
    :param 文字列 省略可 区切り文字: 結合時の区切り文字
    :param 真偽値 省略可 空文字除外: FALSEなら配列要素が空文字でも結合する、TRUEなら除外
    :param 数値 省略可 開始: 結合範囲の開始位置のインデックス値
    :param 数値 省略可 終了: 結合範囲の終了位置のインデックス値
    :return: 結合後の文字列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            arr = ["foo", "bar", "baz", "qux"]
            print join(arr) // foo bar baz qux
            print join(arr, "+") // foo+bar+baz+qux
            print join(arr, "+", FALSE, 1, 2) // bar+baz

            // 空文字除外
            print join(["hoge", "","fuga"], "&", FALSE) // hoge&&fuga
            print join(["hoge", "","fuga"], "&", TRUE) // hoge&fuga

.. function:: split(文字列, [区切り文字=" ", 空文字除外=FALSE, 数値変換=FALSE, CSV分割=FALSE])

    | 文字列を区切り文字で分割して配列にします

    :param 文字列 文字列: 分割したい文字列
    :param 文字列 省略可 区切り文字: 分割するための区切り、CSV分割が有効の場合最初の一文字のみ使用される

        .. admonition:: 一文字ずつ分割
            :class: tip

            | 区切り文字として空文字を指定すると文字列を一文字ずつ分割できます

    :param 真偽値 省略可 空文字除外: FALSEなら分割後に空文字があっても配列要素とする、TRUEなら除外
    :param 真偽値 省略可 数値変換: TRUEなら分割後の文字列を数値へ変換し、変換できない場合は空文字とする
    :param 真偽値 省略可 CSV分割: TRUEならCSVとして分割する (空文字除外と数値変換は無視される)
    :return: 分割された配列

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            print split("a b c") // [a, b, c]

            // 空文字除外
            print split("a,,b,,c", ",", FALSE) // [a, , b, , c]
            print split("a,,b,,c", ",", TRUE) // [a, b, c]

            // 数値変換
            print split("1,2,f,4,5", ",", FALSE, FALSE) // [1, 2, f, 4, 5]
            print split("1,2,f,4,5", ",", FALSE, TRUE) // [1, 2, , 4, 5]
            // 空文字除外と組み合わせると数値以外を排除できる
            print split("1,2,f,4,5", ",", TRUE, TRUE) // [1, 2, 4, 5]

            // 空文字で分割
            print split("12345", "", FALSE) // [, 1, 2, 3, 4, 5, ]
            print split("12345", "", TRUE)  // [1, 2, 3, 4, 5]

            // CSV分割
            // , で区切られる
            print split('a,b,"c,d",e', ",", , , FALSE) // [a, b, "c, d", e]
            // "" 内を文字列扱いとし中の , では区切らない
            print split('a,b,"c,d",e', ",", , , TRUE)  // [a, b, c,d, e]