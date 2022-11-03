# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

project = 'UWSCR'
copyright = '2022, stuncloud'
author = 'stuncloud'

# -- General configuration ---------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#general-configuration

extensions = []

templates_path = ['_templates']
exclude_patterns = []

language = 'ja'

# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_theme = 'furo'
html_static_path = ['_static']
html_js_files = ['custom.js']

# extensions = ['myst_parser']
# source_suffix = {
#     '.rst': 'restructuredtext',
#     '.md': 'markdown',
# }

# code highlight

from ast import keyword
from string import whitespace
from typing_extensions import Literal
from pygments.lexer import RegexLexer, words, include, bygroups
from pygments.token import Keyword, Name, Whitespace, Comment, String, Number, Punctuation, Operator, Text, Literal

class UwscrLexer(RegexLexer):
    name = 'UWSCR'
    aliases = ['uwscr']
    filenames = ['*.uws']

    tokens = {
        'root': [
            include('comment'),
            include('whitespace'),
            include('string'),
            include('number'),
            (r'\btextblock\b\n', Keyword, 'tbcomment'),
            (r'\b(textblock|textblockex)\b(\s+)(\w+\n)', bygroups(Keyword, Whitespace, Name), 'textblock'),
            (r'(call)(\s+)(url)(\[)([^]]+)(\])', bygroups(Keyword, Whitespace, Keyword, Punctuation, Literal, Punctuation)),
            (words((',', '=', '.', ':', '?', '>', '<', '|', '_', '+', '-', '*', '/', '!', '#')), Operator),
            (words(('(', ')', '[', ']', '{', '}', '@', ';')), Punctuation),
            (words((
                'print', 'for', 'in', 'next', 'endfor',
                'async', 'await',
                'if', 'then', 'else', 'elseif', 'endif',
                'var', 'ref', 'args', 'prms',
                'while', 'wend', 'repeat', 'until',
                'break', 'continue',
            ), suffix=r'\b', prefix=r'\b'), Keyword),
            (words((
                'dim', 'public', 'const',
                'function', 'procedure', 'fend',
                'hashtbl', 'hash', 'endhash', 'enum', 'endenum',
                'select', 'selend', 'with', 'endwith',
                'module', 'endmodule', 'class', 'endclass',
            ), suffix=r'\b', prefix=r'\b'), Keyword.Declaration),
            (r'\b[a-zA-Z_][a-zA-Z_0-9]*\b', Name),
            # (words(('env'), suffix=r'\b', prefix=r'\b'), Name),
        ],
        'string': [
            (r'"[^"]*"', String),
            (r"'[^']*'", String),
        ],
        'whitespace': [
            (r' ', Whitespace),
            (r"　", Whitespace),
        ],
        'comment': [
            (r'//.*\n', Comment),
        ],
        'textblock': [
            (r'\bendtextblock\b', Keyword, '#pop'),
            (r'.+\n', String),
        ],
        'tbcomment': [
            (r'\bendtextblock\b', Keyword, '#pop'),
            (r'.+\n', Comment),
        ],
        'number': [
            (r'\b[0-9]+\b', Number),
            (r'\b\$[0-9a-fA-F]+\b', Number),
            (r'\b[A-Z0-9_]+\b', Number),
            (r'\bNaN+\b', Number),
        ],
    }

from sphinx.highlighting import lexers;
lexers['uwscr'] = UwscrLexer()