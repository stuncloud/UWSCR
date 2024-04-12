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

.. function:: random(n)

    | 0以上n未満の整数をランダムに返す

    .. note:: 指定可能な最大値は2147483646です


    :param 数値 n: 得たい数値の範囲を示す値
    :rtype: 数値
    :return: 得られたランダム値

.. function:: abs(n)

    | 絶対値を得る

    :param 数値 n: 入力値
    :rtype: 数値
    :return: 絶対値

.. function:: zcut(n)

    | マイナス値は0にする

    :param 数値 n: 入力値
    :rtype: 数値
    :return: 0以上の整数

.. function:: int(n)

    | 小数点以下切り落とし

    :param 数値 n: 入力値
    :rtype: 数値
    :return: 整数

.. function:: ceil(n)

    | 小数点以下を正方向に切り上げ

    :param 数値 n: 入力値
    :rtype: 数値
    :return: 整数

.. function:: round(n, [桁=0])

    | 指定桁数で入力値を丸める

    :param 数値 n: 入力値
    :param 数値 省略可 桁: 丸める桁、マイナスなら小数点以下の桁数
    :rtype: 数値
    :return: 整数

.. function:: sqrt(n)

    | 平方根

    :param 数値 n: 入力値
    :rtype: 数値
    :return: 入力値の平方根、入力値がマイナスの場合NaN

.. function:: power(n, e)

    | nをe乗する

    :param 数値 n: 入力値
    :param 数値 e: 指数
    :rtype: 数値
    :return: 数値

.. function:: exp(n)

    | 指数関数

    :param 数値 n: 入力値
    :rtype: 数値
    :return: 数値

.. function:: ln(n)

    | 自然対数

    :param 数値 n: 入力値
    :rtype: 数値
    :return: 数値

.. function:: logn(base, n)

    | baseを底とするnの対数

    :param 数値 base: 底
    :param 数値 n: 値
    :rtype: 対数
    :return: 数値

.. function:: sin(n)

    | サイン

    :param 数値 n: 入力値
    :rtype: 数値
    :return: ラジアン

.. function:: cos(n)

    | コサイン

    :param 数値 n: 入力値
    :rtype: 数値
    :return: ラジアン

.. function:: tan(n)

    | タンジェント

    :param 数値 n: 入力値
    :rtype: 数値
    :return: ラジアン

.. function:: arcsin(n)

    | アークサイン

    :param 数値 n: 入力値
    :rtype: 数値
    :return: ラジアン

.. function:: arccos(n)

    | アークコサイン

    :param 数値 n: 入力値
    :rtype: 数値
    :return: ラジアン

.. function:: arctan(n)

    | アークタンジェント

    :param 数値 n: 入力値
    :rtype: 数値
    :return: ラジアン

