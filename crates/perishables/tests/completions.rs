//! Integration tests for `perishables::completions`.

use std::str::FromStr;

use perishables::completions::{Shell, completions_script, write_completions};

const EXPECTED_SUBCOMMANDS: &[&str] = &[
    "audit",
    "completions",
    "corpus",
    "drift",
    "grade",
    "grade-dbt",
    "grade-ossie",
    "grade-snowflake",
    "grade-databricks",
    "grade-cube",
    "grade-looker",
    "grade-atscale",
];

const EXPECTED_FLAGS: &[&str] = &[
    "--as-of",
    "--format",
    "--output",
    "--no-exec",
    "--timing",
    "--ecosystem",
    "--changed-at",
    "--help",
    "--version",
];

const EXPECTED_ECOSYSTEMS: &[&str] = &[
    "dbt",
    "ossie",
    "snowflake",
    "databricks",
    "cube",
    "looker",
    "atscale",
];

const EXPECTED_FORMATS: &[&str] = &["text", "json", "sarif", "github"];

#[test]
fn shell_parse_display_and_roundtrip() {
    let cases = [
        ("bash", Shell::Bash, "bash"),
        ("  BASH ", Shell::Bash, "bash"),
        ("zsh", Shell::Zsh, "zsh"),
        ("Zsh", Shell::Zsh, "zsh"),
        ("fish", Shell::Fish, "fish"),
        ("FISH", Shell::Fish, "fish"),
        ("powershell", Shell::PowerShell, "powershell"),
        ("PowerShell", Shell::PowerShell, "powershell"),
        ("pwsh", Shell::PowerShell, "powershell"),
    ];
    for (raw, expected, canonical) in cases {
        let parsed = Shell::from_str(raw).expect("valid shell");
        assert_eq!(parsed, expected);
        assert_eq!(parsed.as_str(), canonical);
        assert_eq!(parsed.to_string(), canonical);
        assert_eq!(parsed.to_string().parse::<Shell>(), Ok(expected));
    }

    let err = "tcsh".parse::<Shell>().expect_err("tcsh is unsupported");
    assert_eq!(err.input(), "tcsh");
    assert!(err.to_string().contains("tcsh"));
    assert!(err.to_string().contains("bash, zsh, fish, or powershell"));
}

#[test]
fn all_shells_cover_every_subcommand_flag_format_and_ecosystem() {
    for shell in Shell::ALL {
        let script = completions_script(shell);
        assert!(!script.is_empty(), "{shell} script must not be empty");
        assert!(
            script.ends_with('\n'),
            "{shell} script must end with a trailing newline"
        );

        for sub in EXPECTED_SUBCOMMANDS {
            assert!(
                script.contains(sub),
                "{shell} completion script missing subcommand {sub:?}"
            );
        }
        for flag in EXPECTED_FLAGS {
            assert!(
                script.contains(flag),
                "{shell} completion script missing flag {flag:?}"
            );
        }
        for eco in EXPECTED_ECOSYSTEMS {
            assert!(
                script.contains(eco),
                "{shell} completion script missing ecosystem {eco:?}"
            );
        }
        for fmt_name in EXPECTED_FORMATS {
            assert!(
                script.contains(fmt_name),
                "{shell} completion script missing output format {fmt_name:?}"
            );
        }

        let mut buf = Vec::new();
        write_completions(shell, &mut buf).expect("write completions to memory buffer");
        assert_eq!(
            buf,
            script.as_bytes(),
            "write_completions output must match completions_script for {shell}"
        );
    }
}

#[test]
fn shell_specific_entrypoints_are_present() {
    let bash = completions_script(Shell::Bash);
    assert!(bash.contains("_perishables()"));
    assert!(bash.contains("complete -F _perishables perishables"));

    let zsh = completions_script(Shell::Zsh);
    assert!(zsh.starts_with("#compdef perishables\n"));
    assert!(zsh.contains("_perishables \"$@\""));

    let fish = completions_script(Shell::Fish);
    assert!(fish.contains("complete -c perishables"));
    assert!(fish.contains("__fish_use_subcommand"));
    assert!(fish.contains("__fish_seen_subcommand_from"));

    let pwsh = completions_script(Shell::PowerShell);
    assert!(pwsh.contains("Register-ArgumentCompleter -Native -CommandName perishables"));
    assert!(pwsh.contains("[System.Management.Automation.CompletionResult]::new"));
}
