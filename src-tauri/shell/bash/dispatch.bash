# Dispatch starts bash with this as its --init-file. Load the user's startup
# files the way a login shell would, then put the Dispatch prompt on top.
# Kept to bash 3.2, the version macOS ships.

# Aliases from Settings › Aliases, already quoted by Dispatch: first so the user's
# startup files can use them, and again after them so they win over one of the same name.
[ -n "$DISPATCH_ALIASES" ] && eval "$DISPATCH_ALIASES"
[ -r /etc/profile ] && . /etc/profile
if [ -r "$HOME/.bash_profile" ]; then
  . "$HOME/.bash_profile"
elif [ -r "$HOME/.bash_login" ]; then
  . "$HOME/.bash_login"
elif [ -r "$HOME/.profile" ]; then
  . "$HOME/.profile"
fi

[ -n "$DISPATCH_ALIASES" ] && eval "$DISPATCH_ALIASES"
unset DISPATCH_ALIASES
# With the Dispatch prompt off, Dispatch only starts bash this way for the aliases.
if [ "$DISPATCH_PROMPT" = 0 ]; then
  unset DISPATCH_PROMPT DISPATCH_PROMPT_INLINE
  return
fi
unset DISPATCH_PROMPT

# The Dispatch prompt: Node version, folder, git branch, and changed files,
# printed as plain text. Each chip is wrapped in OSC 6973 marks so Dispatch can
# draw a border and icon around it; the marks take no space on screen.
# More marks split the screen into one block per command: "prompt" where the
# prompt starts, "exec" when a command starts (bash 4.4 and later, through
# PS0), "done;<exit code>" when it ends. "prompt;newline" also tells Dispatch
# that the prompt binds its keys: Shift+Enter, sent as CSI 13;2u, continues the
# command on a new line, CSI 127;9u clears the line being typed (Select All,
# then Backspace), and Option+Left/Right (CSI 1;3D and 1;3C) move by word. bash only says so from 4.4 on, since before that
# Dispatch can't tell when a command is running. "input" marks where the
# command starts, after the chips. "cwd;<folder>" and "path;<PATH>" tell
# Dispatch where to look for what it suggests as you type.
__dispatch_node_key=""
__dispatch_node_ver=""
# What the prompt shows, read through promptvars so a branch name is never run as code.
__dispatch_ver=""
__dispatch_branch=""
__dispatch_code=0
__dispatch_path=""
# Dispatch sets this when commands are typed right after the chips instead of on the line below.
__dispatch_inline=${DISPATCH_PROMPT_INLINE:-0}
unset DISPATCH_PROMPT_INLINE
__dispatch_newline_mark=""

# Hands the last command's exit code back, so PROMPT_COMMAND still sees it after Dispatch reads it.
__dispatch_keep() {
  return "$__dispatch_code"
}

__dispatch_prompt() {
  printf '\e]6973;done;%s\a' "$__dispatch_code"
  # The folder, and PATH when it changes, for Dispatch's suggestions. Git Bash
  # on Windows reports the Windows path, which is what Dispatch can read.
  local here=$PWD
  case $OSTYPE in msys* | cygwin*) here=$(pwd -W 2>/dev/null || printf '%s' "$PWD") ;; esac
  printf '\e]6973;cwd;%s\a' "$here"
  if [ "$PATH" != "$__dispatch_path" ]; then
    __dispatch_path=$PATH
    printf '\e]6973;path;%s\a' "$PATH"
  fi
  local dir=$PWD node_root="" git_root="" ver="" branch="" changes=""
  while true; do
    if [ -z "$node_root" ] && { [ -e "$dir/package.json" ] || [ -e "$dir/.nvmrc" ] || [ -e "$dir/.node-version" ]; }; then
      node_root=$dir
    fi
    [ -z "$git_root" ] && [ -e "$dir/.git" ] && git_root=$dir
    { [ "$dir" = / ] || [ -z "$dir" ]; } && break
    dir=${dir%/*}
    [ -z "$dir" ] && dir=/
  done

  if [ -n "$node_root" ] && command -v node >/dev/null 2>&1; then
    # Version managers switch Node by folder (shims) or PATH (nvm use), so either one changing asks again.
    if [ "$PWD|$PATH" != "$__dispatch_node_key" ]; then
      __dispatch_node_key="$PWD|$PATH"
      __dispatch_node_ver=$(command node --version 2>/dev/null)
    fi
    ver=$__dispatch_node_ver
  fi

  if [ -n "$git_root" ]; then
    local out line oid="" head="" n=0
    if out=$(GIT_OPTIONAL_LOCKS=0 command git --no-optional-locks status --porcelain=v2 --branch 2>/dev/null); then
      while IFS= read -r line; do
        case $line in
          "# branch.oid "*) oid=${line#"# branch.oid "} ;;
          "# branch.head "*) head=${line#"# branch.head "} ;;
          "#"* | "") ;;
          *) n=$((n + 1)) ;;
        esac
      done <<<"$out"
      if [ "$head" = "(detached)" ]; then
        [ "$oid" != "(initial)" ] && branch=${oid:0:7}
      else
        branch=$head
      fi
      changes=$n
    fi
  fi

  __dispatch_ver=$ver
  __dispatch_branch=$branch
  local end='\[\e]6973;end\a\]' p='\n\[\e]6973;prompt'"$__dispatch_newline_mark"'\a\]'
  # The prompt starts with a blank line: Dispatch draws the block's border through the middle of it.
  # A chip is one cell of padding, two for the icon, the text, and one more cell of padding.
  [ -n "$ver" ] && p+='\[\e]6973;chip;node\a\]   \[\e[32m\]${__dispatch_ver}\[\e[39m\] '"$end "
  p+='\[\e]6973;chip;cwd\a\]   \w '"$end "
  [ -n "$branch" ] && p+='\[\e]6973;chip;branch\a\]   \[\e[32m\]${__dispatch_branch}\[\e[39m\] '"$end "
  if [ -n "$changes" ]; then
    local color=90
    [ "$changes" -gt 0 ] && color=33
    p+='\[\e]6973;chip;changes\a\] \[\e['"$color"'m\]± '"$changes"'\[\e[39m\] '"$end "
  fi
  # The command goes on its own line under the chips, unless set to follow them.
  [ "$__dispatch_inline" = 1 ] || p="${p% }\n"
  p+='\[\e]6973;input\a\]'
  shopt -s promptvars
  PS1=$p
}

# Last in line, so the user's own PROMPT_COMMAND can't put its prompt back over this one.
# The exit code is read first, before anything else can change it.
if [ -n "$PROMPT_COMMAND" ]; then
  PROMPT_COMMAND='__dispatch_code=$?; __dispatch_keep'$'\n'"$PROMPT_COMMAND"$'\n'"__dispatch_prompt"
else
  PROMPT_COMMAND='__dispatch_code=$?; __dispatch_prompt'
fi
# PS0 prints as each command starts; bash before 4.4 has no PS0, so its blocks have no run time.
if [ "${BASH_VERSINFO[0]}" -gt 4 ] || { [ "${BASH_VERSINFO[0]}" -eq 4 ] && [ "${BASH_VERSINFO[1]}" -ge 4 ]; }; then
  # A blank line after the command: Dispatch draws the bottom of its card through it.
  PS0="${PS0}"$'\e]6973;exec\a\n'
  # Readline can't show a newline inside the line it edits (it prints ^J), so
  # Shift+Enter continues the command on the next line with a trailing backslash.
  bind '"\e[13;2u": "\C-e \\\C-m"'
  bind '"\e[127;9u": "\C-e\C-u"'
  # Readline's words already end at punctuation.
  bind '"\e[1;3D": backward-word'
  bind '"\e[1;3C": forward-word'
  __dispatch_newline_mark=";newline"
fi
# Pasted text looks like typed text, as in a text field, rather than standing
# out (readline 8.1 and later highlight it; older ones don't know the setting).
bind 'set enable-active-region off' 2>/dev/null || :
