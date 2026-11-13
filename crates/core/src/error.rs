use std::path::PathBuf;

/// Errors that can cross a layer boundary.
///
/// `thiserror` in this crate, `anyhow` only in binaries. Each variant carries
/// enough context to appear in a user-facing toast without a second lookup.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(feature = "db")]
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    #[cfg(feature = "db")]
    #[error("migration failed: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error("not found: {kind} {id}")]
    NotFound { kind: &'static str, id: String },

    #[error("the executable does not exist: {0}")]
    ExecutableMissing(PathBuf),

    #[error("{path} is not an executable this system can launch (found {found})")]
    NotExecutable { path: PathBuf, found: &'static str },

    #[error("no runner is configured for {0}")]
    NoRunner(String),

    #[error("a session is already running for this game")]
    AlreadyRunning,

    #[error("the launch plan is invalid: {0}")]
    Plan(#[from] PlanError),

    #[error("this title requires launching from {0}; it refuses to start elsewhere")]
    RequiresStoreLauncher(&'static str),

    #[error("the game requires administrator rights, which disables playtime tracking")]
    RequiresElevation,

    #[error("i/o error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Reasons a plan cannot be produced. Separate from [`Error`] so the resolver can
/// stay pure and its failures can be asserted with `assert_eq!`.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PlanError {
    #[error("no executable and no rom target was given")]
    NoTarget,
    #[error("the runner kind {0} cannot launch a rom target")]
    RunnerCannotLaunchRom(&'static str),
    #[error("the {0} runner has no program configured")]
    RunnerHasNoExecutable(&'static str),
    #[error("the arguments template references {{{0}}}, which is not available")]
    UnresolvedPlaceholder(&'static str),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
