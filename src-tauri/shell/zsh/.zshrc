# Dispatch: load the user's own ~/.zshrc, then put the Dispatch prompt on top.
DISPATCH_ZDOTDIR=$ZDOTDIR
# Aliases from Settings › Aliases, already quoted by Dispatch: first so the user's
# .zshrc can use them, and again after it so they win over one of the same name.
[[ -n $DISPATCH_ALIASES ]] && eval "$DISPATCH_ALIASES"
if [[ -f "${DISPATCH_USER_ZDOTDIR:-$HOME}/.zshrc" ]]; then
  ZDOTDIR=${DISPATCH_USER_ZDOTDIR:-$HOME}
  . "$ZDOTDIR/.zshrc"
  DISPATCH_USER_ZDOTDIR=$ZDOTDIR
fi
# From here on (.zlogin, and any zsh started inside this one) the user's own files load as usual.
ZDOTDIR=${DISPATCH_USER_ZDOTDIR:-$HOME}
[[ -n $DISPATCH_ALIASES ]] && eval "$DISPATCH_ALIASES"
# With the Dispatch prompt off, Dispatch only starts zsh this way for the aliases.
[[ $DISPATCH_PROMPT != 0 ]] && . "$DISPATCH_ZDOTDIR/dispatch.zsh"
unset DISPATCH_ZDOTDIR DISPATCH_USER_ZDOTDIR DISPATCH_ALIASES DISPATCH_PROMPT
