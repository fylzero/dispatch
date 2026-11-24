# Dispatch: load the user's own ~/.zshrc, then put the Dispatch prompt on top.
DISPATCH_ZDOTDIR=$ZDOTDIR
if [[ -f "${DISPATCH_USER_ZDOTDIR:-$HOME}/.zshrc" ]]; then
  ZDOTDIR=${DISPATCH_USER_ZDOTDIR:-$HOME}
  . "$ZDOTDIR/.zshrc"
  DISPATCH_USER_ZDOTDIR=$ZDOTDIR
fi
# From here on (.zlogin, and any zsh started inside this one) the user's own files load as usual.
ZDOTDIR=${DISPATCH_USER_ZDOTDIR:-$HOME}
. "$DISPATCH_ZDOTDIR/dispatch.zsh"
unset DISPATCH_ZDOTDIR DISPATCH_USER_ZDOTDIR
