" Vim syntax file
" Language: Xenith
" Maintainer: the Xenith repository (editors/nvim/syntax/xenith.vim)
"
" Kept in step with src/tokens.rs (keywords, operators) and
" src/builtins/registry.rs (built-in functions and constants).

if exists("b:current_syntax")
  finish
endif

syntax case match
syntax sync minlines=50

" --- Comments -------------------------------------------------------------
syn keyword xenithTodo contained TODO FIXME NOTE HACK XXX
syn match xenithComment "#.*$" contains=xenithTodo,@Spell

" --- Keywords -------------------------------------------------------------
syn keyword xenithDeclaration let const
" `struct`, `type` and `method` hand off to the name that follows, so the name
" is highlighted as a definition whatever its capitalisation.
syn keyword xenithDeclaration struct skipwhite nextgroup=xenithTypeDef
syn keyword xenithDeclaration type skipwhite nextgroup=xenithTypeDef
syn keyword xenithDeclaration method skipwhite nextgroup=xenithFunction
syn match xenithTypeDef contained "\<\w\+\>"
syn match xenithFunction contained "\<\w\+\>"

syn keyword xenithConditional when otherwise
" `or when` lexes as a single token, so highlight the pair rather than a bare `or`.
syn match   xenithConditional "\<or\_s\+when\>"
syn keyword xenithRepeat for while in
syn keyword xenithStatement release panic
syn keyword xenithBranch skip stop
syn keyword xenithInclude grab from export
syn keyword xenithConversion as

" --- Types ----------------------------------------------------------------
syn keyword xenithType int float string bytes bool list map
" PascalCase is the convention for structs and type aliases. Requiring a
" lowercase letter keeps SCREAMING_SNAKE constants out of this group.
syn match xenithUserType "\<\u\w*\l\w*\>"
syn match xenithUserConstant "\<\u[A-Z0-9_]*\>"

" A name alone on a line before a colon is a struct field. Map keys are quoted
" and so cannot match, and `let x: int` is ruled out by the leading keyword.
syn match xenithField "^\s*\zs\w\+\ze\s*:"

" --- Literals -------------------------------------------------------------
syn keyword xenithBoolean true false
syn keyword xenithNull null
syn keyword xenithConstant TRUE FALSE NULL MATH_PI

" Where two matches start at the same column Vim prefers the one defined last,
" so the float rule must come second or it would lose `1` out of `1.5`.
syn match xenithNumber "\<\d\+\>"
syn match xenithFloat "\<\d\+\.\d*"

" --- Built-ins ------------------------------------------------------------
syn keyword xenithBuiltin echo format ret input input_int clear len
syn keyword xenithBuiltin append pop extend run
syn keyword xenithBuiltin is_num is_str is_list is_fun

" --- Functions ------------------------------------------------------------
syn match xenithFunctionCall "\<\w\+\ze\s*("
" A field or method after a dot reads as a member, not a call.
syn match xenithMember "\.\zs\w\+"

" --- Strings --------------------------------------------------------------
" Any escaped character is accepted; the eight below are the ones that map to
" something other than themselves.
syn match xenithEscape contained "\\[ntr\\\"'{}]"
syn match xenithEscape contained "\\."
syn match xenithBraceEscape contained "{{\|}}"

syn region xenithInterp contained matchgroup=xenithInterpDelim
      \ start="{" end="}"
      \ contains=xenithNumber,xenithFloat,xenithString,xenithBuiltin,xenithBoolean,xenithNull,xenithFunctionCall,xenithMember,xenithOperator

syn region xenithString start=+"+ skip=+\\.+ end=+"+
      \ contains=xenithEscape,xenithBraceEscape,xenithInterp,@Spell
" Backticks are raw: the lexer copies the contents through verbatim, so neither
" escapes nor `{}` interpolation apply. Only `format()` expands braces in one.
syn region xenithRawString start=+`+ end=+`+ contains=@Spell

" --- Operators and punctuation -------------------------------------------
syn match xenithOperator "++\|--\|+=\|-=\|==\|!=\|<=\|>=\|&&\|||\|[-+*/%^<>=!?]"
syn match xenithDelimiter "[,;:]\|::"
" Last, so the arrows win over the single-character operator rule: where two
" matches start together Vim keeps the one defined later.
syn match xenithArrow "->\|=>"

" --- Highlight links ------------------------------------------------------
hi def link xenithComment       Comment
hi def link xenithTodo          Todo

hi def link xenithDeclaration   Keyword
hi def link xenithConditional   Conditional
hi def link xenithRepeat        Repeat
hi def link xenithStatement     Statement
hi def link xenithBranch        Statement
hi def link xenithInclude       Include
hi def link xenithConversion    Keyword

hi def link xenithType          Type
hi def link xenithUserType      Type
hi def link xenithUserConstant  Constant
hi def link xenithTypeDef       Structure
hi def link xenithField         Identifier

hi def link xenithBoolean       Boolean
hi def link xenithNull          Constant
hi def link xenithConstant      Constant
hi def link xenithNumber        Number
hi def link xenithFloat         Float

hi def link xenithBuiltin       Function
hi def link xenithFunction      Function
hi def link xenithFunctionCall  Function
hi def link xenithMember        Identifier

hi def link xenithString        String
hi def link xenithRawString     String
hi def link xenithEscape        SpecialChar
hi def link xenithBraceEscape   SpecialChar
" Interpolated code reads as code, not as more of the string around it.
hi def link xenithInterp        Identifier
hi def link xenithInterpDelim   Special

hi def link xenithArrow         Operator
hi def link xenithOperator      Operator
hi def link xenithDelimiter     Delimiter

let b:current_syntax = "xenith"
