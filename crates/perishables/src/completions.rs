//! Deterministic, zero-dependency shell completion scripts for the `perishables` CLI.
//!
//! Covers every `perishables` subcommand (`audit`, `corpus`, `completions`, `drift`, `grade`,
//! `grade-dbt`, `grade-ossie`, `grade-snowflake`, `grade-databricks`,
//! `grade-cube`, `grade-looker`, and `grade-atscale`) and flag across Bash,
//! Zsh, Fish, and PowerShell without external crates.

use std::fmt;
use std::io::{self, Write};
use std::str::FromStr;

/// Target shell for CLI tab-completion script generation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Shell {
    /// GNU Bash (`complete -F _perishables perishables`).
    Bash,
    /// Zsh (`#compdef perishables`).
    Zsh,
    /// Friendly Interactive Shell (`complete -c perishables`).
    Fish,
    /// PowerShell (`Register-ArgumentCompleter -Native -CommandName perishables`).
    PowerShell,
}

impl Shell {
    /// All supported shells in canonical deterministic order.
    pub const ALL: [Self; 4] = [Self::Bash, Self::Zsh, Self::Fish, Self::PowerShell];

    /// Canonical lowercase identifier for this shell (`bash`, `zsh`, `fish`, or `powershell`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::PowerShell => "powershell",
        }
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when parsing an unsupported shell name via [`Shell::from_str`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseShellError {
    input: String,
}

impl ParseShellError {
    /// Returns the unrecognized shell string that failed to parse.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

impl fmt::Display for ParseShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unsupported shell {:?} (expected bash, zsh, fish, or powershell)",
            self.input
        )
    }
}

impl std::error::Error for ParseShellError {}

impl FromStr for Shell {
    type Err = ParseShellError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            "powershell" | "pwsh" => Ok(Self::PowerShell),
            _ => Err(ParseShellError {
                input: s.to_string(),
            }),
        }
    }
}

/// Return the static completion script for `shell`.
#[must_use]
pub const fn completions_script(shell: Shell) -> &'static str {
    match shell {
        Shell::Bash => BASH_COMPLETIONS,
        Shell::Zsh => ZSH_COMPLETIONS,
        Shell::Fish => FISH_COMPLETIONS,
        Shell::PowerShell => POWERSHELL_COMPLETIONS,
    }
}

/// Write the completion script for `shell` to `out`.
///
/// # Errors
///
/// Returns any I/O error encountered while writing to `out`.
pub fn write_completions<W: Write>(shell: Shell, out: &mut W) -> io::Result<()> {
    out.write_all(completions_script(shell).as_bytes())
}

const BASH_COMPLETIONS: &str = r#"_perishables() {
  local cur prev cmd i word
  COMPREPLY=()
  cur="${COMP_WORDS[COMP_CWORD]}"
  prev=""
  if [ "${COMP_CWORD}" -gt 0 ]; then
    prev="${COMP_WORDS[COMP_CWORD-1]}"
  fi

  local commands="audit completions corpus drift grade grade-dbt grade-ossie grade-snowflake grade-databricks grade-cube grade-looker grade-atscale help"
  local ecosystems="dbt ossie snowflake databricks cube looker atscale"
  local formats="text json sarif github"

  cmd=""
  i=1
  while [ "${i}" -lt "${COMP_CWORD}" ]; do
    word="${COMP_WORDS[i]}"
    case "${word}" in
      --)
        break
        ;;
      -*)
        ;;
      *)
        cmd="${word}"
        break
        ;;
    esac
    i=$((i + 1))
  done

  if [ -z "${cmd}" ]; then
    case "${cur}" in
      -*)
        COMPREPLY=( $(compgen -W "-h --help -V --version" -- "${cur}") )
        ;;
      *)
        COMPREPLY=( $(compgen -W "${commands}" -- "${cur}") )
        ;;
    esac
    return 0
  fi

  case "${prev}" in
    --format)
      if [ "${cmd}" = "corpus" ]; then
        COMPREPLY=( $(compgen -W "text json" -- "${cur}") )
      else
        COMPREPLY=( $(compgen -W "${formats}" -- "${cur}") )
      fi
      return 0
      ;;
    --ecosystem)
      COMPREPLY=( $(compgen -W "${ecosystems}" -- "${cur}") )
      return 0
      ;;
    --output|-o)
      COMPREPLY=( $(compgen -f -- "${cur}") )
      return 0
      ;;
    --as-of|--changed-at)
      return 0
      ;;
  esac

  case "${cmd}" in
    audit)
      if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "--as-of --format --output -o --no-exec --timing -h --help" -- "${cur}") )
      else
        COMPREPLY=( $(compgen -d -- "${cur}") )
      fi
      ;;
    corpus|drift)
      if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "--format --output -o -h --help" -- "${cur}") )
      else
        COMPREPLY=( $(compgen -d -- "${cur}") )
      fi
      ;;
    completions)
      if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "-h --help" -- "${cur}") )
      else
        COMPREPLY=( $(compgen -W "bash zsh fish powershell" -- "${cur}") )
      fi
      ;;
    grade)
      if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "--ecosystem --changed-at --format --output -o -h --help" -- "${cur}") )
      else
        COMPREPLY=( $(compgen -f -- "${cur}") )
      fi
      ;;
    grade-snowflake)
      if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "--changed-at --format --output -o -h --help" -- "${cur}") )
      else
        COMPREPLY=( $(compgen -f -- "${cur}") )
      fi
      ;;
    grade-dbt|grade-ossie|grade-databricks|grade-cube|grade-looker|grade-atscale)
      if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "--format --output -o -h --help" -- "${cur}") )
      else
        COMPREPLY=( $(compgen -f -- "${cur}") )
      fi
      ;;
    help)
      COMPREPLY=( $(compgen -W "audit completions corpus drift grade grade-dbt grade-ossie grade-snowflake grade-databricks grade-cube grade-looker grade-atscale" -- "${cur}") )
      ;;
  esac
  return 0
}

complete -F _perishables perishables
"#;

const ZSH_COMPLETIONS: &str = r#"#compdef perishables

_perishables() {
  local context state state_descr line
  typeset -A opt_args

  local -a commands
  commands=(
    'audit:Audit an Open Knowledge Format bundle at a pinned as-of date'
    'completions:Print a shell completion script'
    'corpus:Run the planted-defect reference corpus gate'
    'drift:Check SHA-256 source drift across an OKF bundle'
    'grade:Grade semantic-layer definitions with --ecosystem or auto-detection'
    'grade-dbt:Grade a dbt semantic manifest (manifest.json or semantic_manifest.json)'
    'grade-ossie:Grade an Apache Ossie model (.json or .yaml)'
    'grade-snowflake:Grade a Snowflake semantic view (.yaml)'
    'grade-databricks:Grade a Databricks Genie serialized space (.json)'
    'grade-cube:Grade a Cube model (meta .json, .yaml, or .js)'
    'grade-looker:Grade a Looker API model (.json) or native LookML project (.lookml)'
    'grade-atscale:Grade an AtScale semantic model export (.json)'
    'help:Print help for a subcommand'
  )

  _arguments -C \
    '(-h --help)'{-h,--help}'[Print help information]' \
    '(-V --version)'{-V,--version}'[Print version information]' \
    '1:command:->cmd' \
    '*::arg:->args' && return 0

  case "${state}" in
    cmd)
      _describe -t commands 'perishables command' commands
      ;;
    args)
      case "${line[1]}" in
        audit)
          _arguments \
            '--as-of=[Pinned RFC 3339 timestamp or YYYY-MM-DD date]:timestamp:' \
            '--format=[Output format]:format:(text json sarif github)' \
            '(-o --output)'{-o,--output=}'[Write report to path outside bundle]:output file:_files' \
            '--no-exec[Skip Python execution of Attested Computations]' \
            '--timing[Include wall-clock elapsed_ms on executed computations]' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '1:bundle directory:_files -/'
          ;;
        corpus)
          _arguments \
            '--format=[Output format]:format:(text json)' \
            '(-o --output)'{-o,--output=}'[Write report to path]:output file:_files' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '1:corpus or fixtures directory:_files -/'
          ;;
        completions)
          _arguments \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '1:shell:(bash zsh fish powershell)'
          ;;
        drift)
          _arguments \
            '--format=[Output format]:format:(text json sarif github)' \
            '(-o --output)'{-o,--output=}'[Write report to path outside bundle]:output file:_files' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '1:bundle directory:_files -/'
          ;;
        grade)
          _arguments \
            '--ecosystem=[Semantic ecosystem]:ecosystem:(dbt ossie snowflake databricks cube looker atscale)' \
            '--changed-at=[Definition last-changed date or epoch seconds]:timestamp:' \
            '--format=[Output format]:format:(text json sarif github)' \
            '(-o --output)'{-o,--output=}'[Write report to path]:output file:_files' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '*:definition file:_files'
          ;;
        grade-snowflake)
          _arguments \
            '--changed-at=[Definition last-changed date or epoch seconds]:timestamp:' \
            '--format=[Output format]:format:(text json sarif github)' \
            '(-o --output)'{-o,--output=}'[Write report to path]:output file:_files' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '1:Snowflake semantic view YAML:_files'
          ;;
        grade-looker)
          _arguments \
            '--format=[Output format]:format:(text json sarif github)' \
            '(-o --output)'{-o,--output=}'[Write report to path]:output file:_files' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '*:Looker JSON or LookML file:_files'
          ;;
        grade-dbt|grade-ossie|grade-databricks|grade-cube|grade-atscale)
          _arguments \
            '--format=[Output format]:format:(text json sarif github)' \
            '(-o --output)'{-o,--output=}'[Write report to path]:output file:_files' \
            '(-h --help)'{-h,--help}'[Print help information]' \
            '1:definition file:_files'
          ;;
        help)
          _describe -t commands 'perishables subcommand' commands
          ;;
      esac
      ;;
  esac
}

_perishables "$@"
"#;

const FISH_COMPLETIONS: &str = r#"# Fish shell completions for perishables
# Supports --as-of, --format, --output, --no-exec, --timing, --ecosystem, --changed-at, --help, and --version.

complete -c perishables -n "__fish_use_subcommand" -s h -l help -d "Print help information (--help)"
complete -c perishables -n "__fish_use_subcommand" -s V -l version -d "Print version information (--version)"

complete -c perishables -f -n "__fish_use_subcommand" -a audit -d "Audit an Open Knowledge Format bundle at a pinned as-of date"
complete -c perishables -f -n "__fish_use_subcommand" -a corpus -d "Run the planted-defect reference corpus gate"
complete -c perishables -f -n "__fish_use_subcommand" -a completions -d "Print a shell completion script"
complete -c perishables -f -n "__fish_use_subcommand" -a drift -d "Check SHA-256 source drift across an OKF bundle"
complete -c perishables -f -n "__fish_use_subcommand" -a grade -d "Grade semantic-layer definitions with --ecosystem or auto-detection"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-dbt -d "Grade a dbt semantic manifest"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-ossie -d "Grade an Apache Ossie model (.json or .yaml)"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-snowflake -d "Grade a Snowflake semantic view (.yaml)"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-databricks -d "Grade a Databricks Genie serialized space (.json)"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-cube -d "Grade a Cube model (.json, .yaml, or .js)"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-looker -d "Grade a Looker API model (.json) or native LookML files (.lookml)"
complete -c perishables -f -n "__fish_use_subcommand" -a grade-atscale -d "Grade an AtScale semantic model export (.json)"
complete -c perishables -f -n "__fish_use_subcommand" -a help -d "Print help for a subcommand"

complete -c perishables -n "__fish_seen_subcommand_from audit" -r -l as-of -d "Pinned RFC 3339 timestamp or YYYY-MM-DD date (--as-of)"
complete -c perishables -n "__fish_seen_subcommand_from audit" -l no-exec -d "Skip Python execution of Attested Computations (--no-exec)"
complete -c perishables -n "__fish_seen_subcommand_from audit" -l timing -d "Include wall-clock elapsed_ms on executed computations (--timing)"

complete -c perishables -n "__fish_seen_subcommand_from grade" -x -l ecosystem -a "dbt ossie snowflake databricks cube looker atscale" -d "Target semantic ecosystem (--ecosystem)"
complete -c perishables -n "__fish_seen_subcommand_from grade grade-snowflake" -r -l changed-at -d "Definition last-changed date or epoch seconds (--changed-at)"

complete -c perishables -n "__fish_seen_subcommand_from corpus" -x -l format -a "text json" -d "Output format (--format)"
complete -c perishables -n "__fish_seen_subcommand_from completions" -x -a "bash zsh fish powershell" -d "Target shell"
complete -c perishables -n "__fish_seen_subcommand_from audit drift grade grade-dbt grade-ossie grade-snowflake grade-databricks grade-cube grade-looker grade-atscale" -x -l format -a "text json sarif github" -d "Output format (--format)"

complete -c perishables -n "__fish_seen_subcommand_from audit corpus drift grade grade-dbt grade-ossie grade-snowflake grade-databricks grade-cube grade-looker grade-atscale" -r -s o -l output -d "Write report to path (--output)"
complete -c perishables -n "__fish_seen_subcommand_from audit completions corpus drift grade grade-dbt grade-ossie grade-snowflake grade-databricks grade-cube grade-looker grade-atscale" -s h -l help -d "Print subcommand help (--help)"
complete -c perishables -f -n "__fish_seen_subcommand_from help" -a "audit completions corpus drift grade grade-dbt grade-ossie grade-snowflake grade-databricks grade-cube grade-looker grade-atscale"
"#;

const POWERSHELL_COMPLETIONS: &str = r#"Register-ArgumentCompleter -Native -CommandName perishables -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commands = @(
        'audit',
        'completions',
        'corpus',
        'drift',
        'grade',
        'grade-dbt',
        'grade-ossie',
        'grade-snowflake',
        'grade-databricks',
        'grade-cube',
        'grade-looker',
        'grade-atscale',
        'help'
    )
    $ecosystems = @('dbt', 'ossie', 'snowflake', 'databricks', 'cube', 'looker', 'atscale')
    $formats = @('text', 'json', 'sarif', 'github')

    $elements = $commandAst.CommandElements | ForEach-Object { $_.Extent.Text }
    $subcommand = $null
    for ($i = 1; $i -lt $elements.Count; $i++) {
        $token = $elements[$i]
        if ($token -eq $wordToComplete -and $i -eq ($elements.Count - 1)) {
            break
        }
        if ($token -eq '--') {
            break
        }
        if (-not $token.StartsWith('-')) {
            $subcommand = $token
            break
        }
    }

    $prev = if ($elements.Count -ge 2) {
        if ($wordToComplete -eq '') { $elements[-1] } elseif ($elements.Count -ge 3) { $elements[-2] } else { '' }
    } else { '' }

    if (-not $subcommand) {
        $candidates = if ($wordToComplete.StartsWith('-')) {
            @('--help', '-h', '--version', '-V')
        } else {
            $commands
        }
        $candidates | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
        return
    }

    if ($prev -eq '--format') {
        $choices = if ($subcommand -eq 'corpus') { @('text', 'json') } else { $formats }
        $choices | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
        return
    }

    if ($prev -eq '--ecosystem') {
        $ecosystems | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
        return
    }

    if ($subcommand -eq 'completions' -and -not $wordToComplete.StartsWith('-')) {
        @('bash', 'zsh', 'fish', 'powershell') | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
        return
    }

    $flags = switch ($subcommand) {
        'audit' { @('--as-of', '--format', '--output', '-o', '--no-exec', '--timing', '--help', '-h') }
        'completions' { @('--help', '-h') }
        'corpus' { @('--format', '--output', '-o', '--help', '-h') }
        'drift' { @('--format', '--output', '-o', '--help', '-h') }
        'grade' { @('--ecosystem', '--changed-at', '--format', '--output', '-o', '--help', '-h') }
        'grade-snowflake' { @('--changed-at', '--format', '--output', '-o', '--help', '-h') }
        'help' { $commands | Where-Object { $_ -ne 'help' } }
        default { @('--format', '--output', '-o', '--help', '-h') }
    }

    if ($subcommand -eq 'help' -or $wordToComplete.StartsWith('-')) {
        $flags | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterName', $_)
        }
    }
}
"#;
