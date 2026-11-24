# Dispatch: load the user's own ~/.zprofile, from their ZDOTDIR.
if [[ -f "${DISPATCH_USER_ZDOTDIR:-$HOME}/.zprofile" ]]; then
  DISPATCH_ZDOTDIR=$ZDOTDIR
  ZDOTDIR=${DISPATCH_USER_ZDOTDIR:-$HOME}
  . "$ZDOTDIR/.zprofile"
  DISPATCH_USER_ZDOTDIR=$ZDOTDIR
  ZDOTDIR=$DISPATCH_ZDOTDIR
fi
