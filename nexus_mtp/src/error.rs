use thiserror::Error;

#[derive(Debug, Error)]
pub enum MtpError {
    #[error("Erro de banco de dados: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Erro de IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("Erro de serialização JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Modelo não encontrado: {0}")]
    ModelNotFound(String),
    #[error("Domínio inválido: '{0}'. Use: rust | infra | security | mlops")]
    InvalidDomain(String),
    #[error("Nenhum documento aprovado encontrado para domínio '{0}'")]
    NoDocuments(String),
    #[error("Treinamento falhou com código {code}: {stderr}")]
    TrainingFailed { code: i32, stderr: String },
    #[error("Modelo precisa ter status 'approved' para deploy. Status atual: '{0}'")]
    NotApproved(String),
    #[error("Adapter não encontrado em: {0}")]
    AdapterNotFound(String),
    #[error("Erro de variável de ambiente: {0}")]
    EnvVar(#[from] std::env::VarError),
    #[error("Erro de UUID: {0}")]
    Uuid(#[from] uuid::Error),
    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for MtpError {
    fn from(e: anyhow::Error) -> Self {
        MtpError::Other(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, MtpError>;
