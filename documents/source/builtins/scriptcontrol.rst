スクリプト制御
==============

実行をブロック
--------------

.. function:: sleep(秒数)

    | 指定した秒数の間スクリプトの実行をブロックします

    :param 数値 秒数: スクリプトの実行を停止する秒数

.. function:: sleep(関数)

    | 関数がTRUEを返す限り実行をブロックします

    :param ユーザー定義関数 関数: 条件判定を行う関数

動的評価
--------

.. function:: eval(構文)

    | 渡された文字列をUWSCRの構文として評価します
    | 式として評価された場合はその結果の値を返します

    :param 文字列 構文: UWSCRの構文を表す文字列
    :return: 式が評価された場合はその実行結果の ``値`` 、文が評価された場合は ``EMPTY``

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            a = 1
            eval("a = 5") // = で代入できる

            for i = 0 to 3
                print a
                eval("if a > 3 then a -= 1 else a += 1") // 単行IF
            next
            print a

エラー発生
----------

.. function:: raise(エラーメッセージ, タイトル=規定のタイトル)

    | 実行時エラーを故意に発生させます

    :param 文字列 エラーメッセージ: エラー内容を示す文字列
    :param 文字列 省略可 タイトル: エラーのタイトル
    :return: なし

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            try
                print 1
                raise("エラーが発生しました")
                print 2
            except
                print TRY_ERRMSG
            endtry

        .. code-block:: powershell

            # 結果
            1
            [ユーザー定義エラー] エラーが発生しました

.. function:: assert_equal(値1, 値2)

    | 2つの値を比較し、一致していない場合は実行時エラーになります

    :param 値 値1: 任意の値
    :param 値 値2: 比較する値
    :return: なし

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            dim a = 5, b = a, c = a * 2
            assert_equal(a, b) // 一致するので何も起こらない
            assert_equal(b, c) // [assert_equalエラー] left: 5; right: 10


タスク
------

.. function:: Task(func, [args, ...])

    | 関数を非同期に実行し、実行中の状態をタスクとして返します

    .. admonition:: await実行した場合
        :class: hint

        | Task関数自体をawaitで実行した場合は関数の終了を待ちその戻り値を返します

    :param 関数 func: 非同期実行させるユーザー定義関数
    :param 値 省略可 args: 関数に渡す引数 (最大20個まで指定可能)
    :rtype: :ref:`task_object`
    :return: 実行中の :ref:`task_object`

.. function:: WaitTask(task)

    | :ref:`task_object` の完了を待ち、関数の戻り値を得ます
    | Promiseに相当する :ref:`remote_object` を受けた場合はそのPromiseの完了を待ち :ref:`remote_object` を返します

    .. admonition:: Promise以外はエラー
        :class: caution

        | :ref:`remote_object` がPromiseではない場合エラーで終了します

    :param タスク task: 未完了の :ref:`task_object`, または :ref:`remote_object`
    :return: :ref:`task_object` として実行していた関数の戻り値、または :ref:`remote_object`

    .. admonition:: サンプルコード

        .. sourcecode:: uwscr

            function MyTask(wait: number)
                for i = 1 to wait
                    sleep(1)
                    print "タスク実行中: " + (wait - i)
                next
                result = "タスク実行完了: <#wait>秒待ちました"
            fend

            t = Task(MyTask, 5)
            print "タスクを開始しました"
            print "タスクは非同期で実行されるため、その間別の処理を行えます"
            print "WaitTaskを呼ぶと処理をブロックし、タスクの完了を待ちます"
            print "タスクが完了すると関数のresult値を得られます"
            print WaitTask(t) // タスク実行完了: 5秒待ちました
