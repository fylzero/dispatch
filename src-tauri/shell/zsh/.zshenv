# Dispatch: load the user's own ~/.zshenv, from their ZDOTDIR.
if [[ -f "${DISPATCH_USER_ZDOTDIR:-$HOME}/.zshenv" ]]; then
  DISPATCH_ZDOTDIR=$ZDOTDIR
  ZDOTDIR=${DISPATCH_USER_ZDOTDIR:-$HOME}
  . "$ZDOTDIR/.zshenv"
  DISPATCH_USER_ZDOTDIR=$ZDOTDIR
  ZDOTDIR=$DISPATCH_ZDOTDIR
fi
