数学関数
========

.. function:: isnan(数値)

    | 値が ``NaN`` であるかどうかを調べる

    :param 数値 数値: 調べる値
    :return: ``NaN`` であればTRUE

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            print IsNan(NaN)  // True
            n = NaN
            print IsNan(n)    // True
            print IsNan(1)    // False
            print IsNan("あ") // False
