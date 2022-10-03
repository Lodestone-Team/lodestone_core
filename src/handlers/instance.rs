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
use crate::traits::{InstanceInfo, Supported, Unsupported};

use super::util::{is_authorized, try_auth};
use crate::json_store::permission::Permission::{self};
use crate::{
    implementations::minecraft,
    traits::{t_server::State, Error, ErrorInner},
    AppState,
};

pub async fn list_instance(
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<InstanceInfo>>, Error> {
    let mut list_of_configs: Vec<InstanceInfo> = join_all(state.instances.lock().await.iter().map(
        |(_, instance)| async move {
            // want id, name, playercount, maxplayer count, port, state and type
            let instance = instance.lock().await;
            instance.get_instance_info().await
        },
    ))
    .await
    .into_iter()
    .collect();

    list_of_configs.sort_by(|a, b| a.creation_time.cmp(&b.creation_time));

    Ok(Json(list_of_configs))
}

pub async fn instance_info(
    Path(uuid): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<InstanceInfo>, Error> {
    Ok(Json(
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
            .get_instance_info()
            .await,
    ))
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
    if name.len() > 100 {
        return Err(Error {
            inner: ErrorInner::MalformedRequest,
            detail: "Name must not be longer than 100 characters".to_string(),
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

    tokio::task::spawn({
        let uuid = uuid.clone();
        async move {
            let minecraft_instance = match minecraft::Instance::new(
                setup_config.clone(),
                state.event_broadcaster.clone(),
                Some(query.key),
            )
            .await
            {
                Ok(v) => v,
                Err(_) => {
                    tokio::fs::remove_dir_all(setup_config.path)
                        .await
                        .map_err(|e| Error {
                            inner: ErrorInner::FailedToRemoveFileOrDir,
                            detail: format!(
                            "Instance creation failed. Failed to clean up instance directory: {}",
                            e
                        ),
                        });
                    return;
                }
            };
            let mut port_allocator = state.port_allocator.lock().await;
            port_allocator.add_port(setup_config.port);
            state
                .instances
                .lock()
                .await
                .insert(uuid.clone(), Arc::new(Mutex::new(minecraft_instance)));
        }
    });
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
    drop(users);
    let instance_list = state.instances.lock().await;
    let mut instance = instance_list
        .get(&uuid)
        .ok_or(Error {
            inner: ErrorInner::InstanceNotFound,
            detail: "".to_string(),
        })?
        .lock()
        .await;
    if !port_scanner::local_port_available(instance.port().await as u16) {
        return Err(Error {
            inner: ErrorInner::PortInUse,
            detail: format!("Port {} is already in use", instance.port().await),
        });
    }
    instance.start().await?;
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
        .stop()
        .await?;
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
        .kill()
        .await?;
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
        .send_command(&query.command)
        .await
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
    Ok(Json(json!(
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
            .state()
            .await
    )))
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
        .get_player_count()
        .await
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
        .get_max_player_count()
        .await
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
        .get_player_list()
        .await
    {
        Supported(v) => Ok(Json(v)),
        Unsupported => Err(Error {
            inner: ErrorInner::UnsupportedOperation,
            detail: "".to_string(),
        }),
    }
}

pub async fn get_instance_name(
    Extension(state): Extension<AppState>,
    Path(uuid): Path<String>,
) -> Result<Json<String>, Error> {
    Ok(Json(
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
            .name()
            .await
            .to_string(),
    ))
}

pub async fn set_instance_name(
    Extension(state): Extension<AppState>,
    Path(uuid): Path<String>,
    Json(name): Json<String>,
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
        .set_name(name)
        .await?;
    Ok(Json(json!("ok")))
}

pub async fn get_instance_description(
    Extension(state): Extension<AppState>,
    Path(uuid): Path<String>,
) -> Result<Json<String>, Error> {
    Ok(Json(
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
            .description()
            .await
            .to_string(),
    ))
}

pub async fn set_instance_description(
    Extension(state): Extension<AppState>,
    Path(uuid): Path<String>,
    Json(description): Json<String>,
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
        .set_description(description)
        .await?;
    Ok(Json(json!("ok")))
}
