use crate::AppState;
use axum::{extract::Path, Extension, Json};

/// Check whether a port is in use
/// Note: this function is not cheap
pub async fn is_port_in_use(
    Extension(state): Extension<AppState>,
    Path(port): Path<u32>,
) -> Json<bool> {
    Json(state.port_allocator.lock().await.is_port_in_use(port))
}

/// Check whether a name is in use
/// Note: this function is not cheap
pub async fn is_name_in_use(
    Extension(state): Extension<AppState>,
    Path(name): Path<String>,
) -> Json<bool> {
    for (_, instance) in state.instances.lock().await.iter() {
        if instance.lock().await.name().await == name {
            return Json(true);
        }
    }
    Json(false)
}