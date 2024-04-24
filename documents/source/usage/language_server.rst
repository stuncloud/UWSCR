Language Server機能について
===========================

UWSCRは ``Language Server Protocol`` 準拠のLanguage Serverを実装しています。任意のエディタでUWSCR用のLanguage Clientを実装することでLanguage ServerからUWSCRのコーティング支援を受けられます。

Language Clientとの通信方法
---------------------------

UWSCRのLanguage ServerはClientの子プロセスとして動作し、標準入出力によりLanguage Server Protocolでの通信を行います。

Clientからは以下のコマンドでLanguage Serverを起動します。

.. code-block:: powershell

    uwscr --language-server

Language Serverが提供する機能
-----------------------------

Diagnostics
^^^^^^^^^^^

構文解析を行い解析エラー情報をClientに送信します。UWSCRでは以下の通知(notification)にがあった場合に ``textDocument/publishDiagnostics`` 通知を送信します。

- ``textDocument/didOpen`` : .uwsファイルが開かれたときにServerに送信される通知
- ``textDocument/didSave`` : ファイルを保存したときにServerに送信される通知

Completion
^^^^^^^^^^

コード補完機能です。Clientの ``textDocument/completion`` 通知に対して以下を返します。

- 組み込み定数
- 組み込み関数
- コードスニペット

