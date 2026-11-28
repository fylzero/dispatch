# Dispatch runs this after your profile when it starts PowerShell with
# Suggestions on. It keeps your prompt and adds marks that take no space:
# "cwd;<folder>" and "path;<PATH>" tell Dispatch where to look for what it
# suggests as you type, and "line" marks where the command starts.
if (-not (Test-Path variable:global:__DispatchPrompt)) {
  $global:__DispatchPrompt = $function:prompt
  function global:prompt {
    $e = [char]27
    $b = [char]7
    $text = & $global:__DispatchPrompt
    $dir = $executionContext.SessionState.Path.CurrentFileSystemLocation.ProviderPath
    "$e]6973;cwd;$dir$b$e]6973;path;$env:PATH$b$text$e]6973;line$b"
  }
}
