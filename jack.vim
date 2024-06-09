" Vim syntax file
" Language: rosa
" Maintainer: ***REMOVED*** ***REMOVED***

if exists("b:current_syntax")
  finish
endif

set nospell
setlocal iskeyword+=#
let b:current_syntax = "jack"

" syn match   jackStaticFun /fun\s+static/ nextgroup=jackIdent skipwhite
" syn keyword jackKeyword struct nextgroup=jackIdent skipwhite
" syn keyword jackKeyword impl nextgroup=jackIdent skipwhite
" syn keyword jackKeyword fun nextgroup=jackIdent skipwhite
" syn keyword jackKeyword #define nextgroup=jackIdent skipwhite
" syn keyword jackKeyword const nextgroup=jackType,jackArray,jackCustomType skipwhite
syn keyword jackIntrinsic print printc println readln drop push pop get set len
syn keyword jackKeyword macro if else switch typeswitch while default dowhile times do done loop for cmperr
syn keyword jackStackOp swap drop dup dupx1
syn keyword jackType    list anylist int string any
syn keyword jackBool    true false

" taken from https://github.com/vim/vim/blob/master/runtime/syntax/c.vim
syn match jackInclude display "^\s*\zs\%(%:\|#\)\s*include\>\s*"
syn match jackOp /(==)|<|>|(<=)|(>=)/
syn match jackStore '=' nextgroup=jackIdent

syn region jackLineComment start='//' end='$'
syn region jackBlockComment start='/\*' end='\*/'

" syn region jackStruct   start=/struct\s*\w*\s*{/ end='}' fold transparent contains=jackKeyword,jackStructItem
" syn region jackArray    start='\[' end='\]' transparent contained contains=jackType,jackArray,jackIdent
syn region jackString   start='"' skip=/\\"/ end='"' contains=@Spell

" https://github.com/uiiaoo/java-syntax.vim/blob/master/syntax_body/java/syntax/pattern.vim
sy match  jackChar     "'[^\\']'"
sy match  jackChar     "'\\[btnfr\"'\\]'"
sy match  jackChar     "\v'\\%(\o{1,3}|u\x{4})'"

syn match jackIdent         /[a-zA-Z_\$][a-zA-Z0-9_\$]*/ contained
" syn match jackStructItem    /^[a-zA-Z_\$][a-zA-Z0-9_\$]*\:/ nextgroup=jackCustomType skipwhite
" syn match jackCustomType    /[\#a-zA-Z][a-zA-Z0-9_\$\#]*/ contained
syn match jackInt           /\v\c<\d%(\d|_*\d)*L=>/
syn match jackFloat         /\v\c<\d%(\d|_*\d)*%(E[+-]=\d%(\d|_*\d)*[FD]=|[FD])>/
syn match jackFloat         /\v\c<\d%(\d|_*\d)*\.%(\d%(\d|_*\d)*)=%(E[+-]=\d%(\d|_*\d)*)=[FD]=/
syn match jackFloat         /\v\c\.\d%(\d|_*\d)*%(E[+-]=\d%(\d|_*\d)*)=[FD]=/
" syn match jackChar          /'([^']|\\.)'/
" syn match jackReturn        '->' nextgroup=jackCustomType skipwhite



hi def link jackInclude         Include
hi def link jackIntrinsic       Statement
hi def link jackKeyword         Keyword
hi def link jackStackOp         Operator
hi def link jackOp              Operator
hi def link jackLineComment     Comment
hi def link jackBlockComment    Comment
hi def link jackType            Type
hi def link jackString          String
hi def link jackInt             Number
hi def link jackFloat           Float
hi def link jackBool            Boolean
hi def link jackChar            Character
hi def link jackIdent           Identifier
hi def link jackStore           Operator

