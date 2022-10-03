use axum::{extract::Path, Extension, Json};
use serde_json::{json, Value};

use crate::{
    traits::{t_manifest::Manifest, Error, ErrorInner},
    AppState,
};

pub async fn get_instance_manifest(
    Path(uuid): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Manifest>, Error> {
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
            .get_manifest()
            .await,
    ))
}

pub async fn get_instance_port(
    Path(uuid): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<u32>, Error> {
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
            .port()
            .await,
    ))
}

pub async fn set_instance_port(
    Path(uuid): Path<String>,
    Extension(state): Extension<AppState>,
    Json(port): Json<u32>,
) -> Result<Json<String>, Error> {
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
        .set_port(port)
        .await
        .ok_or(Error {
            inner: ErrorInner::UnsupportedOperation,
            detail: "".to_string(),
        })??;
    Ok(Json("ok".to_string()))
}
