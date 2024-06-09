#!/bin/bash
mkdir -p ~/.config/lvim/syntax
mkdir -p ~/.config/lvim/ftdetect
cp jack.vim ~/.config/lvim/syntax/jack.vim
echo "au BufRead,BufNewFile *.jack set filetype=jack" > ~/.config/lvim/ftdetect/jack.vim
