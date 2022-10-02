use std::sync::Arc;

use axum::extract::Query;
use axum::{extract::Path, Extension, Json};
use axum_auth::AuthBearer;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;
use ts_rs::TS;

use crate::implementations::minecraft::{Flavour, SetupConfig};
use crate::prelude::PATH_TO_INSTANCES;
use crate::traits::{Supported, Unsupported};

use super::util::{is_authorized, try_auth};
use crate::json_store::permission::Permission::{self};
use crate::{
    implementations::minecraft,
    traits::{t_server::State, Error, ErrorInner},
    AppState,
};

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export)]
pub struct InstanceListInfo {
    pub uuid: String,
    pub name: String,
    pub port: u32,
    pub description: String,
    pub game_type: String,
    pub flavour: String,
    pub state: State,
    pub player_count: u32,
    pub max_player_count: u32,
    pub creation_time: i64,
}

pub async fn list_instance(
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<InstanceListInfo>>, Error> {
    let mut list_of_configs: Vec<InstanceListInfo> = join_all(
        state
            .instances
            .lock()
            .await
            .iter()
            .map(|(_, instance)| async move {
                // want id, name, playercount, maxplayer count, port, state and type
                let instance = instance.lock().await;

                InstanceListInfo {
                    uuid: instance.uuid().await,
                    name: instance.name().await,
                    port: instance.port().await,
                    description: instance.description().await,
                    game_type: instance.game_type().await,
                    flavour: instance.flavour().await,
                    state: instance.state().await,
                    player_count: instance.get_player_count().await.unwrap_or(0),
                    max_player_count: instance.get_max_player_count().await.unwrap_or(0),
                    creation_time: instance.creation_time().await,
                }
            }),
    )
    .await
    .into_iter()
    .collect();

    list_of_configs.sort_by(|a, b| a.creation_time.cmp(&b.creation_time));

    Ok(Json(list_of_configs))
}

#[derive(Deserialize)]
pub struct InstanceCreateQuery {
    pub key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MinecraftSetupConfigPrimitive {
    pub name: String,
    pub version: String,
    pub flavour: Flavour,
    pub port: u32,
    pub cmd_args: Option<Vec<String>>,
    pub description: Option<String>,
    pub fabric_loader_version: Option<String>,
    pub fabric_installer_version: Option<String>,
    pub min_ram: Option<u32>,
    pub max_ram: Option<u32>,
    pub auto_start: Option<bool>,
    pub restart_on_crash: Option<bool>,
    pub timeout_last_left: Option<u32>,
    pub timeout_no_activity: Option<u32>,
    pub start_on_connection: Option<bool>,
    pub backup_period: Option<u32>,
}

impl From<MinecraftSetupConfigPrimitive> for SetupConfig {
    fn from(config: MinecraftSetupConfigPrimitive) -> Self {
        SetupConfig {
            name: config.name.clone(),
            version: config.version,
            flavour: config.flavour,
            port: config.port,
            cmd_args: config.cmd_args,
            description: config.description,
            fabric_loader_version: config.fabric_loader_version,
            fabric_installer_version: config.fabric_installer_version,
            min_ram: config.min_ram,
            max_ram: config.max_ram,
            auto_start: config.auto_start,
            restart_on_crash: config.restart_on_crash,
            timeout_last_left: config.timeout_last_left,
            timeout_no_activity: config.timeout_no_activity,
            start_on_connection: config.start_on_connection,
            backup_period: config.backup_period,
            game_type: "minecraft".to_string(),
            uuid: uuid::Uuid::new_v4().to_string(),
            path: PATH_TO_INSTANCES.with(|path| path.join(config.name)),
        }
    }
}
pub async fn create_minecraft_instance(
    Extension(state): Extension<AppState>,
    Json(mut primitive_setup_config): Json<MinecraftSetupConfigPrimitive>,
    Query(query): Query<InstanceCreateQuery>,
) -> Result<Json<String>, Error> {
    primitive_setup_config.name = sanitize_filename::sanitize(&primitive_setup_config.name);
    let setup_config: SetupConfig = primitive_setup_config.into();
    let name = setup_config.name.clone();
    if name.is_empty() {
        return Err(Error {
            inner: ErrorInner::MalformedRequest,
            detail: "Name must not be empty".to_string(),
        });
    }
    for (_, instance) in state.instances.lock().await.iter() {
        let instance = instance.lock().await;
        if instance.name().await == name {
            return Err(Error {
                inner: ErrorInner::MalformedRequest,
                detail: "Instance with name already exists".to_string(),
            });
        }
    }

    let uuid = setup_config.uuid.clone();

    let minecraft_instance = match minecraft::Instance::new(
        setup_config.clone(),
        state.event_broadcaster.clone(),
        Some(query.key),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tokio::fs::remove_dir_all(setup_config.path)
                .await
                .map_err(|e| Error {
                    inner: ErrorInner::FailedToRemoveFileOrDir,
                    detail: format!(
                        "Instance creation failed. Failed to clean up instance directory: {}",
                        e
                    ),
                })?;
            return Err(e);
        }
    };
    let mut port_allocator = state.port_allocator.lock().await;
    port_allocator.add_port(setup_config.port);
    state
        .instances
        .lock()
        .await
        .insert(uuid.clone(), Arc::new(Mutex::new(minecraft_instance)));
    Ok(Json(uuid))
}

pub async fn remove_instance(
    Extension(state): Extension<AppState>,
    Path(uuid): Path<String>,
) -> Result<Json<Value>, Error> {
    let mut instances = state.instances.lock().await;
    if let Some(instance) = instances.get(&uuid) {
        let instance_lock = instance.lock().await;
        if !(instance_lock.state().await == State::Stopped) {
            Err(Error {
                inner: ErrorInner::InstanceStarted,
                detail: "Instance is running, cannot remove".to_string(),
            })
        } else {
            tokio::fs::remove_dir_all(instance_lock.path().await)
                .await
                .map_err(|e| Error {
                    inner: ErrorInner::FailedToRemoveFileOrDir,
                    detail: format!("Could not remove instance: {}", e),
                })?;

            state
                .port_allocator
                .lock()
                .await
                .deallocate(instance_lock.port().await);
            drop(instance_lock);
            instances.remove(&uuid);
            Ok(Json(json!("OK")))
        }
    } else {
        Err(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: format!("Instance with uuid {} does not exist", uuid),
        })
    }
}

pub async fn start_instance(
    Extension(state): Extension<AppState>,
    Path(uuid): Path<String>,
    AuthBearer(token): AuthBearer,
) -> Result<Json<Value>, Error> {
    let users = state.users.lock().await;
    let requester = try_auth(&token, users.get_ref()).ok_or(Error {
        inner: ErrorInner::PermissionDenied,
        detail: "".to_string(),
    })?;
    if !is_authorized(&requester, &uuid, Permission::CanStartInstance) {
        return Err(Error {
            inner: ErrorInner::PermissionDenied,
            detail: "Not authorized to start instance".to_string(),
        });
    }
    state
        .instances
        .lock()
        .await
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await
        .start().await?;
    Ok(Json(json!("ok")))
}

pub async fn stop_instance(
    Extension(state): Extension<AppState>,
    Path(uuid): Path<String>,
) -> Result<Json<Value>, Error> {
    state
        .instances
        .lock()
        .await
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await
        .stop().await?;
    Ok(Json(json!("ok")))
}

pub async fn kill_instance(
    Extension(state): Extension<AppState>,
    Path(uuid): Path<String>,
) -> Result<Json<Value>, Error> {
    state
        .instances
        .lock()
        .await
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await
        .kill().await?;
    Ok(Json(json!("ok")))
}

#[derive(Deserialize)]
pub struct SendCommandQuery {
    command: String,
}

pub async fn send_command(
    Extension(state): Extension<AppState>,
    Path(uuid): Path<String>,
    Query(query): Query<SendCommandQuery>,
) -> Result<Json<Value>, Error> {
    match state
        .instances
        .lock()
        .await
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await
        .send_command(&query.command).await
    {
        Supported(v) => v.map(|_| Json(json!("ok"))),
        Unsupported => Err(Error {
            inner: ErrorInner::UnsupportedOperation,
            detail: "".to_string(),
        }),
    }
}

pub async fn get_instance_state(
    Extension(state): Extension<AppState>,
    Path(uuid): Path<String>,
) -> Result<Json<Value>, Error> {
    Ok(Json(json!(state
        .instances
        .lock()
        .await
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await
        .state().await)))
}

pub async fn get_player_count(
    Extension(state): Extension<AppState>,
    Path(uuid): Path<String>,
) -> Result<Json<u32>, Error> {
    match state
        .instances
        .lock()
        .await
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await
        .get_player_count().await
    {
        Supported(v) => Ok(Json(v)),
        Unsupported => Err(Error {
            inner: ErrorInner::UnsupportedOperation,
            detail: "".to_string(),
        }),
    }
}

pub async fn get_max_player_count(
    Extension(state): Extension<AppState>,
    Path(uuid): Path<String>,
) -> Result<Json<u32>, Error> {
    match state
        .instances
        .lock()
        .await
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await
        .get_max_player_count().await
    {
        Supported(v) => Ok(Json(v)),
        Unsupported => Err(Error {
            inner: ErrorInner::UnsupportedOperation,
            detail: "".to_string(),
        }),
    }
}

pub async fn get_player_list(
    Extension(state): Extension<AppState>,
    Path(uuid): Path<String>,
) -> Result<Json<Vec<Value>>, Error> {
    match state
        .instances
        .lock()
        .await
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await
        .get_player_list().await
    {
        Supported(v) => Ok(Json(v)),
        Unsupported => Err(Error {
            inner: ErrorInner::UnsupportedOperation,
            detail: "".to_string(),
        }),
    }
}
