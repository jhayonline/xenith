" Indent rules for Xenith.
"
" Braces delimit every block, so C indenting is almost right out of the box.
" The one adjustment is that Xenith has no statement terminator, which makes
" cindent treat each line as an unfinished statement and over-indent.

if exists("b:did_indent")
  finish
endif
let b:did_indent = 1

setlocal indentexpr=GetXenithIndent()
setlocal indentkeys=0{,0},0),0],!^F,o,O,e

setlocal nolisp
setlocal nosmartindent

if exists("*GetXenithIndent")
  finish
endif

function! s:StripTrailingComment(line) abort
  " Only strip a `#` that is outside string delimiters, so a `#` inside
  " "a # b" does not truncate the line.
  let l:result = ''
  let l:quote = ''
  let l:index = 0
  while l:index < strlen(a:line)
    let l:char = a:line[l:index]
    if l:quote !=# ''
      if l:char ==# '\' && l:quote ==# '"'
        let l:result .= a:line[l:index : l:index + 1]
        let l:index += 2
        continue
      endif
      if l:char ==# l:quote
        let l:quote = ''
      endif
    elseif l:char ==# '"' || l:char ==# '`'
      let l:quote = l:char
    elseif l:char ==# '#'
      break
    endif
    let l:result .= l:char
    let l:index += 1
  endwhile
  return substitute(l:result, '\s\+$', '', '')
endfunction

function! GetXenithIndent() abort
  let l:current = v:lnum
  let l:previous = prevnonblank(l:current - 1)
  if l:previous == 0
    return 0
  endif

  let l:previous_text = s:StripTrailingComment(getline(l:previous))
  let l:current_text = s:StripTrailingComment(getline(l:current))
  let l:indent = indent(l:previous)

  " Opening a block indents the lines inside it.
  if l:previous_text =~# '[{[(]$'
    let l:indent += shiftwidth()
  endif

  " A closing bracket lines up with the line that opened it.
  if l:current_text =~# '^\s*[}\])]'
    let l:indent -= shiftwidth()
  endif

  return l:indent < 0 ? 0 : l:indent
endfunction
