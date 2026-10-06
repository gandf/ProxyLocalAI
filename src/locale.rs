use serde::Deserialize;

#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Fr,
    En,
}

#[derive(Clone, Copy)]
pub enum Message {
    ConfigInvalid,
    ConfigMissing,
    DefaultsUsed,
    ListenFailed,
    ServerListening,
    ProgramPathMissing,
    ProgramWaitFailed,
    Program,
    ProgramExitedRestarting,
    ProgramRestarting,
    ProgramStopFailed,
    ProgramStopWaitFailed,
    ProgramCheckFailed,
    ProgramStarted,
    ProgramStartFailed,
    RequestReceived,
    RequestRewritten,
    InvalidRequestBody,
    InvalidRequest,
    UpstreamUnavailable,
    UpstreamError,
    Attempt,
    Empty,
    Bytes,
    Truncated,
}

pub fn text(language: Language, message: Message) -> &'static str {
    use Language::{En, Fr};
    use Message::*;

    match (language, message) {
        (Fr, ConfigInvalid) => "Configuration invalide",
        (En, ConfigInvalid) => "Invalid configuration",
        (Fr, ConfigMissing) => "Configuration absente",
        (En, ConfigMissing) => "Configuration file not found",
        (Fr, DefaultsUsed) => "valeurs par défaut",
        (En, DefaultsUsed) => "using default values",
        (Fr, ListenFailed) => "Impossible d'écouter sur",
        (En, ListenFailed) => "Unable to listen on",
        (Fr, ServerListening) => "Proxy à l'écoute",
        (En, ServerListening) => "Proxy listening",
        (Fr, ProgramPathMissing) => "Programme activé mais aucun chemin n'est renseigné",
        (En, ProgramPathMissing) => "Program is enabled but no executable path is configured",
        (Fr, ProgramWaitFailed) => "Erreur en attendant le programme",
        (En, ProgramWaitFailed) => "Error waiting for program",
        (Fr, Program) => "Programme",
        (En, Program) => "Program",
        (Fr, ProgramExitedRestarting) => "terminé; relancement",
        (En, ProgramExitedRestarting) => "exited; restarting",
        (Fr, ProgramRestarting) => "Relancement du programme",
        (En, ProgramRestarting) => "Restarting program",
        (Fr, ProgramStopFailed) => "Impossible d'arrêter le programme",
        (En, ProgramStopFailed) => "Unable to stop program",
        (Fr, ProgramStopWaitFailed) => "Erreur en attendant l'arrêt du programme",
        (En, ProgramStopWaitFailed) => "Error waiting for program to stop",
        (Fr, ProgramCheckFailed) => "Impossible de vérifier le programme",
        (En, ProgramCheckFailed) => "Unable to check program",
        (Fr, ProgramStarted) => "Programme démarré",
        (En, ProgramStarted) => "Program started",
        (Fr, ProgramStartFailed) => "Impossible de démarrer le programme",
        (En, ProgramStartFailed) => "Unable to start program",
        (Fr, RequestReceived) => "reçue",
        (En, RequestReceived) => "received",
        (Fr, RequestRewritten) => "après remplacements",
        (En, RequestRewritten) => "after replacements",
        (Fr, InvalidRequestBody) => "Corps de requête invalide",
        (En, InvalidRequestBody) => "Invalid request body",
        (Fr, InvalidRequest) => "Requête invalide",
        (En, InvalidRequest) => "Invalid request",
        (Fr, UpstreamUnavailable) => "Backend indisponible",
        (En, UpstreamUnavailable) => "Upstream unavailable",
        (Fr, UpstreamError) => "Erreur du backend",
        (En, UpstreamError) => "Upstream error",
        (Fr, Attempt) => "tentative",
        (En, Attempt) => "attempt",
        (Fr, Empty) => "VIDE",
        (En, Empty) => "EMPTY",
        (Fr, Bytes) => "octets",
        (En, Bytes) => "bytes",
        (Fr, Truncated) => "tronqué",
        (En, Truncated) => "truncated",
    }
}
