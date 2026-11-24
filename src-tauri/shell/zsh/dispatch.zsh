# The Dispatch prompt: Node version, folder, git branch, and changed files,
# printed as plain text. Each chip is wrapped in OSC 6973 marks so Dispatch can
# draw a border and icon around it; the marks take no space on screen.

typeset -g __dispatch_node_key="" __dispatch_node_ver=""
# What the prompt shows, read through prompt_subst so a branch or folder name is
# never run as code; % is doubled so it isn't read as a prompt escape either.
typeset -g __dispatch_ver="" __dispatch_branch=""
setopt prompt_subst

__dispatch_prompt() {
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
  # A chip is one cell of padding, two for the icon, the text, and one more cell of padding.
  [[ -n $ver ]] && p+="%{${e}]6973;chip;node${b}%}   %F{green}\${__dispatch_ver}%f ${end} "
  p+="%{${e}]6973;chip;cwd${b}%}   %~ ${end} "
  [[ -n $branch ]] && p+="%{${e}]6973;chip;branch${b}%}   %F{green}\${__dispatch_branch}%f ${end} "
  if [[ -n $changes ]]; then
    local color=8
    (( changes > 0 )) && color=yellow
    p+="%{${e}]6973;chip;changes${b}%} %F{${color}}± ${changes}%f ${end} "
  fi
  setopt prompt_subst
  PROMPT=$p
  RPROMPT=""
}

# Last in line, so a theme's own precmd can't put its prompt back over this one.
precmd_functions=(${precmd_functions:#__dispatch_prompt} __dispatch_prompt)
