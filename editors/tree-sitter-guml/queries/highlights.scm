;; Capture names, kept in step with `guml highlight`'s classes.
;;
;; The mapping is deliberately the same one `crates/guml-fmt/src/highlight.rs` uses, so a document
;; coloured by tree-sitter and the same document coloured by the language server agree. Where they do
;; not, the compiler is right.

(page "page" @keyword)
(type_decl "type" @keyword)
(state_decl ["state" "store"] @keyword)
(data_decl "data" @keyword)
(def_decl "def" @keyword)
(escape_block ["js" "raw"] @keyword)

(tag) @type
(attribute name: (identifier) @property)

;; A bare word in a positional slot is either a label or a modifier, and nothing lexical distinguishes
;; them — only the registry does. So they share one capture here, and the language server refines it via
;; semantic tokens from `guml highlight`.
(element (identifier) @variable.parameter)

(binding) @variable
(expression) @variable
(action) @function
(action_body) @function

(string) @string
(number) @number
(boolean) @constant.builtin
(route) @string.special
(anchor) @label
(method) @keyword.operator

(prose) @string
(content) @string
(content_line) @string
(raw_line) @none

(comment) @comment
