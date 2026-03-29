use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitnookError {
    #[error("Not inside a git repository")]
    NotInGitRepo,

    #[error("No gitnooks found. Run 'gitnook init' first.")]
    NoGitnooksFound,

    #[error("gitnook '{0}' does not exist. Run 'gitnook list' to see all gitnooks.")]
    GitnookNotFound(String),

    #[error("'{0}' is not tracked by gitnook '{1}'")]
    FileNotTracked(String, String),

    #[error("'{0}' is already tracked by gitnook '{1}'")]
    FileAlreadyTracked(String, String),

    #[error("'{0}' does not exist")]
    FileNotFound(String),

    #[error("'{0}' is outside the git repository")]
    FileOutsideRepo(String),

    #[error("gitnook '{0}' already exists. Run 'gitnook list' to see all gitnooks.")]
    GitnookAlreadyExists(String),
}
