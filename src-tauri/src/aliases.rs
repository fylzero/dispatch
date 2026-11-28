//! Aliases from Settings › Terminal, defined in every new tab after the user's
//! own startup files so they win over one of the same name. Each shell gets
//! its own kind: `alias` in zsh and bash, a function in PowerShell (its
//! aliases can't hold arguments), and a doskey macro in cmd. Whatever is typed
//! after the alias goes after its command.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Alias {
    pub name: String,
    pub command: String,
}

/// Aliases under a name of the user's choosing, only to keep them in order
/// in Settings; the shells see one list, group after group.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AliasGroup {
    pub name: String,
    pub aliases: Vec<Alias>,
}

/// Every group's aliases in order, so a later one of the same name wins.
pub fn flatten(groups: &[AliasGroup]) -> Vec<Alias> {
    groups
        .iter()
        .flat_map(|group| group.aliases.iter().cloned())
        .collect()
}

/// A name every shell takes as a plain word: letters, digits, `_`, `-` and
/// `.`, and no leading `-`, which would read as an option.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// The aliases a shell can be given: a valid name and a one-line command.
/// Rows still being filled in, or broken by hand in the JSON, are left out.
pub fn usable(aliases: &[Alias]) -> impl Iterator<Item = (&str, &str)> {
    aliases.iter().filter_map(|alias| {
        let name = alias.name.trim();
        let command = alias.command.trim();
        (valid_name(name) && !command.is_empty() && !command.chars().any(char::is_control))
            .then_some((name, command))
    })
}

/// The names, for suggestions.
pub fn names(aliases: &[Alias]) -> Vec<String> {
    usable(aliases).map(|(name, _)| name.to_string()).collect()
}

/// zsh and bash: `alias -- name='command'`, run with eval by the startup files.
pub fn posix(aliases: &[Alias]) -> String {
    usable(aliases)
        .map(|(name, command)| format!("alias -- {name}='{}'\n", command.replace('\'', r"'\''")))
        .collect()
}

/// PowerShell: a global function per alias. Each one is parsed on its own, so
/// a command PowerShell can't read only loses that alias, with a warning,
/// and not the rest of the startup script. A built-in alias of the same name
/// (ls, cd) is removed first, since aliases run before functions.
pub fn powershell(aliases: &[Alias]) -> String {
    usable(aliases)
        .map(|(name, command)| {
            let body = format!("function global:{name} {{ {command} @args }}").replace('\'', "''");
            format!(
                "Remove-Item -Force -ErrorAction SilentlyContinue 'Alias:{name}'\n\
                 try {{ . ([scriptblock]::Create('{body}')) }} catch {{ Write-Warning ('Dispatch alias {name}: {{0}}' -f $_) }}\n"
            )
        })
        .collect()
}

/// cmd: a doskey macro file, `name=command $*`. `$` is doubled so it stays
/// itself rather than starting a doskey code.
pub fn doskey(aliases: &[Alias]) -> String {
    usable(aliases)
        .map(|(name, command)| format!("{name}={} $*\r\n", command.replace('$', "$$")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alias(name: &str, command: &str) -> Alias {
        Alias {
            name: name.into(),
            command: command.into(),
        }
    }

    #[test]
    fn flattens_groups_in_order() {
        let groups = [
            AliasGroup {
                name: "git".into(),
                aliases: vec![alias("gs", "git status"), alias("x", "one")],
            },
            AliasGroup::default(),
            AliasGroup {
                name: "".into(),
                aliases: vec![alias("x", "two")],
            },
        ];
        assert_eq!(
            flatten(&groups),
            [
                alias("gs", "git status"),
                alias("x", "one"),
                alias("x", "two")
            ]
        );
    }

    #[test]
    fn checks_names() {
        assert!(valid_name("gs"));
        assert!(valid_name("git-up"));
        assert!(valid_name("k8s.ctx_2"));
        assert!(!valid_name(""));
        assert!(!valid_name("-x"));
        assert!(!valid_name("g s"));
        assert!(!valid_name("a=b"));
        assert!(!valid_name("a'b"));
        assert!(!valid_name("a;b"));
    }

    #[test]
    fn leaves_out_rows_that_cannot_be_defined() {
        let list = [
            alias(" gs ", " git status "),
            alias("", "ls"),
            alias("x", "  "),
            alias("bad name", "ls"),
            alias("two", "echo a\necho b"),
        ];
        assert_eq!(usable(&list).collect::<Vec<_>>(), [("gs", "git status")]);
        assert_eq!(names(&list), ["gs"]);
    }

    #[test]
    fn quotes_for_zsh_and_bash() {
        let list = [alias("gs", "git status"), alias("hi", "echo 'it''s' $HOME")];
        assert_eq!(
            posix(&list),
            "alias -- gs='git status'\nalias -- hi='echo '\\''it'\\'''\\''s'\\'' $HOME'\n"
        );
    }

    #[test]
    fn writes_powershell_functions() {
        let script = powershell(&[alias("say", "Write-Output 'hi'")]);
        assert_eq!(
            script,
            "Remove-Item -Force -ErrorAction SilentlyContinue 'Alias:say'\n\
             try { . ([scriptblock]::Create('function global:say { Write-Output ''hi'' @args }')) } catch { Write-Warning ('Dispatch alias say: {0}' -f $_) }\n"
        );
    }

    #[test]
    fn writes_doskey_macros() {
        assert_eq!(
            doskey(&[alias("ll", "dir /w"), alias("cost", "echo $5")]),
            "ll=dir /w $*\r\ncost=echo $$5 $*\r\n"
        );
    }
}
