# The Dispatch prompt: Node version, folder, git branch, and changed files,
# printed as plain text. Each chip is wrapped in OSC 6973 marks so Dispatch can
# draw a border and icon around it; the marks take no space on screen.
# More marks split the screen into one block per command: "prompt" where the
# prompt starts, "exec" when a command starts (followed by a blank line), "done;<exit code>" when it ends.
# "prompt;newline" also tells Dispatch that the prompt binds its keys:
# Shift+Enter, sent as CSI 13;2u, adds a line instead of running the command,
# CSI 127;9u clears the whole command (Select All, then Backspace), and
# Option+Left/Right (CSI 1;3D and 1;3C) move by word, stopping at punctuation.
# "input" marks where the command starts, after the chips. "cwd;<folder>" and
# "path;<PATH>" tell Dispatch where to look for what it suggests as you type.

typeset -g __dispatch_node_key="" __dispatch_node_ver=""
# What the prompt shows, read through prompt_subst so a branch or folder name is
# never run as code; % is doubled so it isn't read as a prompt escape either.
typeset -g __dispatch_ver="" __dispatch_branch=""
typeset -gi __dispatch_ran=0
typeset -g __dispatch_path=""
# Dispatch sets this when commands are typed right after the chips instead of on the line below.
typeset -gi __dispatch_inline=${DISPATCH_PROMPT_INLINE:-0}
unset DISPATCH_PROMPT_INLINE
setopt prompt_subst

__dispatch_newline() {
  LBUFFER+=$'\n'
}
zle -N __dispatch_newline
bindkey -M emacs $'\e[13;2u' __dispatch_newline
bindkey -M viins $'\e[13;2u' __dispatch_newline

__dispatch_clear() {
  BUFFER=""
  CURSOR=0
}
zle -N __dispatch_clear
bindkey -M emacs $'\e[127;9u' __dispatch_clear
bindkey -M viins $'\e[127;9u' __dispatch_clear

# Words end at punctuation, as in macOS text fields, so a path is several
# words. Only these keys see the narrower WORDCHARS; Ctrl+W and the rest keep yours.
__dispatch_word_left() {
  local WORDCHARS='_'
  zle .backward-word
}
__dispatch_word_right() {
  local WORDCHARS='_'
  zle .emacs-forward-word
}
zle -N __dispatch_word_left
zle -N __dispatch_word_right
bindkey -M emacs $'\e[1;3D' __dispatch_word_left
bindkey -M viins $'\e[1;3D' __dispatch_word_left
bindkey -M emacs $'\e[1;3C' __dispatch_word_right
bindkey -M viins $'\e[1;3C' __dispatch_word_right

# Pasted text looks like typed text, as in a text field, rather than standing out.
zle_highlight=(${zle_highlight:#paste:*} paste:none)

__dispatch_preexec() {
  __dispatch_ran=1
  # A blank line after the command: Dispatch draws the bottom of its card through it.
  print -n $'\e]6973;exec\a\n'
}

__dispatch_prompt() {
  local -i code=$?
  if (( __dispatch_ran )); then
    __dispatch_ran=0
    print -n $'\e]6973;done;'"$code"$'\a'
  fi
  # The folder, and PATH when it changes, for Dispatch's suggestions.
  print -rn -- $'\e]6973;cwd;'"$PWD"$'\a'
  if [[ $PATH != $__dispatch_path ]]; then
    __dispatch_path=$PATH
    print -rn -- $'\e]6973;path;'"$PATH"$'\a'
  fi
  local dir=$PWD node_root="" git_root="" ver="" branch="" changes=""
  while true; do
    [[ -z $node_root && ( -e $dir/package.json || -e $dir/.nvmrc || -e $dir/.node-version ) ]] && node_root=$dir
    [[ -z $git_root && -e $dir/.git ]] && git_root=$dir
    [[ $dir == / || -z $dir ]] && break
    dir=${dir:h}
  done

  if [[ -n $node_root ]] && (( $+commands[node] )); then
    # Version managers switch Node by folder (shims) or PATH (nvm use), so either one changing asks again.
    if [[ "$PWD|$PATH" != $__dispatch_node_key ]]; then
      __dispatch_node_key="$PWD|$PATH"
      __dispatch_node_ver=$(command node --version 2>/dev/null)
    fi
    ver=$__dispatch_node_ver
  fi

  if [[ -n $git_root ]]; then
    local out line oid head
    local -i n=0
    if out=$(GIT_OPTIONAL_LOCKS=0 command git --no-optional-locks status --porcelain=v2 --branch 2>/dev/null); then
      for line in ${(f)out}; do
        case $line in
          ("# branch.oid "*) oid=${line#"# branch.oid "} ;;
          ("# branch.head "*) head=${line#"# branch.head "} ;;
          ("#"*) ;;
          (*) n+=1 ;;
        esac
      done
      if [[ $head == "(detached)" ]]; then
        [[ $oid != "(initial)" ]] && branch=${oid[1,7]}
      else
        branch=$head
      fi
      changes=$n
    fi
  fi

  __dispatch_ver=${ver//\%/%%}
  __dispatch_branch=${branch//\%/%%}
  local e=$'\e' b=$'\a' p=""
  local end="%{${e}]6973;end${b}%}"
  # A blank line first: Dispatch draws the block's border through the middle of it.
  p=$'\n'"%{${e}]6973;prompt;newline${b}%}"
  # A chip is one cell of padding, two for the icon, the text, and one more cell of padding.
  [[ -n $ver ]] && p+="%{${e}]6973;chip;node${b}%}   %F{green}\${__dispatch_ver}%f ${end} "
  p+="%{${e}]6973;chip;cwd${b}%}   %~ ${end} "
  [[ -n $branch ]] && p+="%{${e}]6973;chip;branch${b}%}   %F{green}\${__dispatch_branch}%f ${end} "
  if [[ -n $changes ]]; then
    local color=8
    (( changes > 0 )) && color=yellow
    p+="%{${e}]6973;chip;changes${b}%} %F{${color}}± ${changes}%f ${end} "
  fi
  # The command goes on its own line under the chips, unless set to follow them.
  (( __dispatch_inline )) || p="${p% }"$'\n'
  p+="%{${e}]6973;input${b}%}"
  setopt prompt_subst
  PROMPT=$p
  RPROMPT=""
}

# Last in line, so a theme's own precmd can't put its prompt back over this one.
precmd_functions=(${precmd_functions:#__dispatch_prompt} __dispatch_prompt)
preexec_functions=(${preexec_functions:#__dispatch_preexec} __dispatch_preexec)
