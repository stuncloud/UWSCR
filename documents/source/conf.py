# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

project = 'UWSCR'
copyright = '2023, stuncloud'
author = 'stuncloud'
version = '1.1.1'
html_title = f'{project} {version}'

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

extensions = [
    'sphinx-favicon',
    'sphinx.ext.githubpages',
]

favicons = [
    {
        "rel": "icon",
        "static-file": "MAINICON_0016-0256_light.ico",
        "type": "image/vnd.microsoft.icon",
    }
]

# extensions = ['myst_parser']
# source_suffix = {
#     '.rst': 'restructuredtext',
#     '.md': 'markdown',
# }

# code highlight

from typing_extensions import Literal
from pygments.lexer import RegexLexer, words, include, bygroups
from pygments.token import Keyword, Name, Whitespace, Comment, String, Number, Punctuation, Operator, Literal

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
            (words((',', '=', '.', ':', '?', '>', '<', '|', '+', '-', '*', '/', '!', '#')), Operator),
            (words(('(', ')', '[', ']', '{', '}', '@', ';')), Punctuation),
            (words((
                'print', 'for', 'in', 'next', 'endfor',
                'async', 'await',
                'if', 'then', 'else', 'elseif', 'endif',
                'var', 'ref', 'args', 'prms',
                'while', 'wend', 'repeat', 'until',
                'break', 'continue',
                'try', 'except', 'finally', 'endtry'
            ), suffix=r'\b', prefix=r'\b'), Keyword),
            (words((
                'dim', 'public', 'const',
                'function', 'procedure', 'fend',
                'hashtbl', 'hash', 'endhash', 'enum', 'endenum',
                'select', 'selend', 'case', 'default', 'with', 'endwith',
                'module', 'endmodule', 'class', 'endclass', 'def_dll', 'struct', 'endstruct'
            ), suffix=r'\b', prefix=r'\b'), Keyword.Declaration),
            (r'\b[a-zA-Z_][a-zA-Z_0-9]*\b', Name),
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
            (r'//-', Comment), # dummy
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
            (r'\$\b[0-9a-fA-F]+\b', Number),
            (r'\bNaN+\b', Number),
        ],
    }

from sphinx.highlighting import lexers;
lexers['uwscr'] = UwscrLexer()