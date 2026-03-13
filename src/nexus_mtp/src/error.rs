use thiserror::Error;

#[derive(Debug, Error)]
pub enum MtpError {
    #[error("Erro de banco de dados: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Erro de IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("Erro de serializacao JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Erro HTTP: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Modelo nao encontrado: {0}")]
    ModelNotFound(String),
    #[error("Dominio invalido: '{0}'. Use: rust | infra | security | mlops")]
    InvalidDomain(String),
    #[error("Nenhum documento aprovado encontrado para dominio '{0}'")]
    NoDocuments(String),
    #[error("Treinamento falhou com codigo {code}: {stderr}")]
    TrainingFailed { code: i32, stderr: String },
    #[error("Modelo precisa ter status 'approved' para deploy. Status atual: '{0}'")]
    NotApproved(String),
    #[error("Benchmark ainda nao executado para o modelo.")]
    BenchmarkMissing,
    #[error("Benchmark abaixo do minimo ({score:.3} < {min_score:.3}).")]
    BenchmarkBelowThreshold { score: f32, min_score: f32 },
    #[error("Adapter nao encontrado em: {0}")]
    AdapterNotFound(String),
    #[error("Erro de variavel de ambiente: {0}")]
    EnvVar(#[from] std::env::VarError),
    #[error("Erro de UUID: {0}")]
    Uuid(#[from] uuid::Error),
    #[error("Gate da Etapa A nao satisfeito: {0}")]
    StageAGateNotSatisfied(String),
    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for MtpError {
    fn from(e: anyhow::Error) -> Self {
        MtpError::Other(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, MtpError>;
