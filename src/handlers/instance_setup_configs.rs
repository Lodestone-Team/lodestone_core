use crate::error::Error;
use crate::implementations::generic;
use crate::implementations::minecraft_java;
use crate::minecraft_java::FlavourKind;
use crate::traits::t_configurable::manifest::SetupManifest;
use crate::traits::t_configurable::GameType;
use crate::AppState;
use axum::extract::Path;
use axum::routing::get;
use axum::Json;
use axum::Router;
use axum::routing::put;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

#[allow(clippy::enum_variant_names)]
#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub enum HandlerGameType {
    MinecraftJavaVanilla,
    MinecraftJavaFabric,
    MinecraftJavaForge,
    MinecraftJavaPaper,
    MinecraftBedrock,
}

impl From<HandlerGameType> for GameType {
    fn from(value: HandlerGameType) -> Self {
        match value {
            HandlerGameType::MinecraftJavaVanilla => Self::MinecraftJava,
            HandlerGameType::MinecraftJavaFabric => Self::MinecraftJava,
            HandlerGameType::MinecraftJavaForge => Self::MinecraftJava,
            HandlerGameType::MinecraftJavaPaper => Self::MinecraftJava,
            HandlerGameType::MinecraftBedrock => Self::MinecraftBedrock,
        }
    }
}

impl From<HandlerGameType> for FlavourKind {
    fn from(value: HandlerGameType) -> Self {
        match value {
            HandlerGameType::MinecraftJavaVanilla => Self::Vanilla,
            HandlerGameType::MinecraftJavaFabric => Self::Fabric,
            HandlerGameType::MinecraftJavaForge => Self::Forge,
            HandlerGameType::MinecraftJavaPaper => Self::Paper,
            _ => Self::Vanilla, // not sure what pattern works best here
        }
    }
}

pub async fn get_available_games() -> Json<Vec<HandlerGameType>> {
    Json(vec![
        HandlerGameType::MinecraftJavaVanilla,
        HandlerGameType::MinecraftJavaFabric,
        HandlerGameType::MinecraftJavaForge,
        HandlerGameType::MinecraftJavaPaper,
        HandlerGameType::MinecraftBedrock,
    ])
}

pub async fn get_setup_manifest(
    Path(game_type): Path<HandlerGameType>,
) -> Result<Json<SetupManifest>, Error> {
    minecraft_java::MinecraftJavaInstance::setup_manifest(&game_type.into())
        .await
        .map(Json)
}

#[derive(Deserialize)]
pub struct GenericSetupManifestBody {
    pub url: String,
}

pub async fn get_generic_setup_manifest(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(body): Json<GenericSetupManifestBody>,
) -> Result<Json<SetupManifest>, Error> {
    generic::GenericInstance::setup_manifest(&body.url, state.macro_executor)
        .await
        .map(Json)
}


pub fn get_instance_setup_config_routes(appstate: AppState) -> Router {
    Router::new()
        .route("/games", get(get_available_games))
        .route("/setup_manifest/:game_type", get(get_setup_manifest))
        .route(
            "/generic_setup_manifest",
            put(get_generic_setup_manifest),
        )
        .with_state(appstate)
}
