; Special identifiers
;--------------------

([
 (type_decl name: (ident))
 (enum_decl name: (ident))
 (type_ident name: (ident))
]) @type

; Function and method definitions
;--------------------------------

(fn_decl
  name: (ident) @function)
(fn_param
  name: (ident) @variable.parameter)
(type_method
  name: (ident) @function.method)

; Function and method calls
;--------------------------

(call_expr
  fn: [
    (ident) @function
    (member_expr property: (ident) @function.method)
    (arrow_expr property: (ident) @function.method)
    (static_expr property: (ident) @function.method)
  ])

; Properties
;-----------
;
;(property_identifier) @property

; Literals
;---------

(this) @variable.builtin

[
  (true)
  (false)
  (null)
] @constant.builtin

[
  (doc)
] @comment

[
  (string)
] @string

(number) @number

; Annotations
;------------
(annotation) @constant.builtin

; Tokens
;-------

(string_substitution
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

(type_ident (optional)) @punctualtion.special

(string_substitution
  "${" @punctuation.special
  (expr) @embedded
  "}" @punctuation.special)

[
  ";"
  "."
  ","
  "->"
  "::"
] @punctuation.delimiter

[
  "-"
  "--"
  "+"
  "++"
  "*"
  "/"
  "%"
  "<"
  "<="
  "="
  "=="
  "!"
  "!="
  ">"
  ">="
  "^"
  "&&"
  "||"
  "??"
  "?="
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket

[
  "as"
  "at"
  "is"
  "break"
  "catch"
  "type"
  "var"
  "continue"
  "do"
  "else"
  "extends"
  "for"
  "fn"
  "if"
  "in"
  "return"
  "native"
  "static"
  "abstract"
  "private"
  "throw"
  "try"
  "while"
  "enum"
  "typeof"
] @keyword
