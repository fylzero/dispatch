# Dispatch starts bash with this as its --init-file. Load the user's startup
# files the way a login shell would, then put the Dispatch prompt on top.
# Kept to bash 3.2, the version macOS ships.

[ -r /etc/profile ] && . /etc/profile
if [ -r "$HOME/.bash_profile" ]; then
  . "$HOME/.bash_profile"
elif [ -r "$HOME/.bash_login" ]; then
  . "$HOME/.bash_login"
elif [ -r "$HOME/.profile" ]; then
  . "$HOME/.profile"
fi

# The Dispatch prompt: Node version, folder, git branch, and changed files,
# printed as plain text. Each chip is wrapped in OSC 6973 marks so Dispatch can
# draw a border and icon around it; the marks take no space on screen.
__dispatch_node_key=""
__dispatch_node_ver=""
# What the prompt shows, read through promptvars so a branch name is never run as code.
__dispatch_ver=""
__dispatch_branch=""

__dispatch_prompt() {
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
  local end='\[\e]6973;end\a\]' p=""
  # A chip is one cell of padding, two for the icon, the text, and one more cell of padding.
  [ -n "$ver" ] && p+='\[\e]6973;chip;node\a\]   \[\e[32m\]${__dispatch_ver}\[\e[39m\] '"$end "
  p+='\[\e]6973;chip;cwd\a\]   \w '"$end "
  [ -n "$branch" ] && p+='\[\e]6973;chip;branch\a\]   \[\e[32m\]${__dispatch_branch}\[\e[39m\] '"$end "
  if [ -n "$changes" ]; then
    local color=90
    [ "$changes" -gt 0 ] && color=33
    p+='\[\e]6973;chip;changes\a\] \[\e['"$color"'m\]± '"$changes"'\[\e[39m\] '"$end "
  fi
  shopt -s promptvars
  PS1=$p
}

# Last in line, so the user's own PROMPT_COMMAND can't put its prompt back over this one.
if [ -n "$PROMPT_COMMAND" ]; then
  PROMPT_COMMAND="$PROMPT_COMMAND"$'\n'"__dispatch_prompt"
else
  PROMPT_COMMAND="__dispatch_prompt"
fi
