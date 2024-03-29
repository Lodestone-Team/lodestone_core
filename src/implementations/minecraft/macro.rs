use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use async_trait::async_trait;
use color_eyre::eyre::{eyre, Context};
use deno_core::{anyhow, op, OpState};

use crate::{
    error::Error,
    events::{CausedBy, EventInner},
    macro_executor::{self, MacroPID, SpawnResult, WorkerOptionGenerator},
    traits::{
        t_macro::{HistoryEntry, MacroEntry, TMacro, TaskEntry},
        t_server::TServer,
    },
};

use super::MinecraftInstance;

#[op]
async fn send_stdin(state: Rc<RefCell<OpState>>, cmd: String) -> Result<(), anyhow::Error> {
    let instance = state.borrow().borrow::<MinecraftInstance>().clone();
    instance.send_command(&cmd, CausedBy::Unknown).await?;
    Ok(())
}

#[op]
async fn send_rcon(state: Rc<RefCell<OpState>>, cmd: String) -> Result<String, anyhow::Error> {
    let instance = state.borrow().borrow::<MinecraftInstance>().clone();
    let ret = instance.send_rcon(&cmd).await?;
    Ok(ret)
}

#[op]
async fn on_event(
    state: Rc<RefCell<OpState>>,
    event: String,
) -> Result<Option<String>, anyhow::Error> {
    let instance = state.borrow().borrow::<MinecraftInstance>().clone();
    let mut event_rx = instance.event_broadcaster.subscribe();
    if event == "playerMessage" {
        while let Ok(event) = event_rx.recv().await {
            if let EventInner::InstanceEvent(inner) = event.event_inner {
                if let crate::events::InstanceEventInner::PlayerMessage {
                    player,
                    player_message,
                } = inner.instance_event_inner
                {
                    return Ok(Some(
                        serde_json::json!(
                         {
                             "player": player,
                             "message": player_message
                         }
                        )
                        .to_string(),
                    ));
                }
            }
        }
    } else if event == "playersJoined" {
        while let Ok(event) = event_rx.recv().await {
            if let EventInner::InstanceEvent(inner) = event.event_inner {
                if let crate::events::InstanceEventInner::PlayerChange { players_joined, .. } =
                    inner.instance_event_inner
                {
                    if !players_joined.is_empty() {
                        return Ok(Some(
                            serde_json::json!({ "players": players_joined }).to_string(),
                        ));
                    }
                    continue;
                }
            }
        }
    } else if event == "playersLeft" {
        while let Ok(event) = event_rx.recv().await {
            if let EventInner::InstanceEvent(inner) = event.event_inner {
                if let crate::events::InstanceEventInner::PlayerChange { players_left, .. } =
                    inner.instance_event_inner
                {
                    if !players_left.is_empty() {
                        return Ok(Some(
                            serde_json::json!(
                             {
                                 "playersLeft": players_left,
                             }
                            )
                            .to_string(),
                        ));
                    } else {
                        continue;
                    }
                }
            }
        }
    } else if event == "playersChanged" {
        while let Ok(event) = event_rx.recv().await {
            if let EventInner::InstanceEvent(inner) = event.event_inner {
                if let crate::events::InstanceEventInner::PlayerChange { player_list, .. } =
                    inner.instance_event_inner
                {
                    return Ok(Some(
                        serde_json::json!(
                         {
                             "players": player_list,
                         }
                        )
                        .to_string(),
                    ));
                }
            }
        }
    }
    todo!()
}

pub fn resolve_macro_invocation(path_to_macro: &Path, macro_name: &str) -> Option<PathBuf> {
    let ts_macro = path_to_macro.join(macro_name).with_extension("ts");
    let js_macro = path_to_macro.join(macro_name).with_extension("js");

    let macro_folder = path_to_macro.join(macro_name);

    if ts_macro.is_file() {
        return Some(ts_macro);
    } else if js_macro.is_file() {
        return Some(js_macro);
    } else if macro_folder.is_dir() {
        // check if index.ts exists
        let index_ts = macro_folder.join("index.ts");
        let index_js = macro_folder.join("index.js");
        if index_ts.exists() {
            return Some(index_ts);
        } else if index_js.exists() {
            return Some(index_js);
        }
    }
    None
}

pub struct MinecraftMainWorkerGenerator {
    instance: MinecraftInstance,
}

impl MinecraftMainWorkerGenerator {
    pub fn new(instance: MinecraftInstance) -> Self {
        Self { instance }
    }
}

impl WorkerOptionGenerator for MinecraftMainWorkerGenerator {
    fn generate(&self) -> deno_runtime::worker::WorkerOptions {
        let ext = deno_core::Extension::builder("minecraft_deno_extension_builder")
            .ops(vec![
                send_stdin::decl(),
                send_rcon::decl(),
                on_event::decl(),
            ])
            .state({
                let instance = self.instance.clone();
                move |state| {
                    state.put(instance);
                }
            })
            .force_op_registration()
            .build();
        deno_runtime::worker::WorkerOptions {
            extensions: vec![ext],
            module_loader: Rc::new(macro_executor::TypescriptModuleLoader::default()),
            ..Default::default()
        }
    }
}

#[async_trait]
impl TMacro for MinecraftInstance {
    async fn get_macro_list(&self) -> Result<Vec<MacroEntry>, Error> {
        let mut ret = Vec::new();
        for entry in
            (std::fs::read_dir(&self.path_to_macros).context("Failed to read macro dir")?).flatten()
        {
            // if the entry is a file, check if it has the .ts or .js extension
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "ts" || ext == "js" {
                        ret.push(MacroEntry {
                            last_run: self.macro_name_to_last_run.lock().await.get(&name).cloned(),
                            name,
                            path,
                        })
                    }
                }
            } else if path.is_dir() {
                // check if index.ts or index.js exists
                let index_ts = path.join("index.ts");
                let index_js = path.join("index.js");
                if index_ts.exists() || index_js.exists() {
                    ret.push(MacroEntry {
                        last_run: self.macro_name_to_last_run.lock().await.get(&name).cloned(),
                        name,
                        path,
                    })
                }
            }
        }
        ret.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(ret)
    }

    async fn get_task_list(&self) -> Result<Vec<TaskEntry>, Error> {
        let mut ret = Vec::new();
        for (pid, task_entry) in self.pid_to_task_entry.lock().await.iter() {
            if self.macro_executor.get_macro_status(*pid).await.is_none() {
                ret.push(task_entry.clone());
            }
        }
        ret.sort_by(|a, b| a.creation_time.cmp(&b.creation_time));
        Ok(ret)
    }

    async fn get_history_list(&self) -> Result<Vec<HistoryEntry>, Error> {
        let mut ret = Vec::new();
        for (pid, task_entry) in self.pid_to_task_entry.lock().await.iter() {
            if let Some(exit_status) = self.macro_executor.get_macro_status(*pid).await {
                ret.push(HistoryEntry {
                    task: task_entry.clone(),
                    exit_status,
                });
            }
        }
        ret.sort_by(|a, b| b.exit_status.time().cmp(&a.exit_status.time()));
        Ok(ret)
    }

    async fn delete_macro(&mut self, name: &str) -> Result<(), Error> {
        crate::util::fs::remove_file(self.path_to_macros.join(name)).await?;
        Ok(())
    }

    async fn create_macro(&mut self, name: &str, content: &str) -> Result<(), Error> {
        crate::util::fs::write_all(self.path_to_macros.join(name), content.as_bytes().to_vec())
            .await
    }

    async fn run_macro(
        &mut self,
        name: &str,
        args: Vec<String>,
        caused_by: CausedBy,
    ) -> Result<TaskEntry, Error> {
        let path_to_macro = resolve_macro_invocation(&self.path_to_macros, name)
            .ok_or_else(|| eyre!("Failed to resolve macro invocation for {}", name))?;

        let main_worker_generator = MinecraftMainWorkerGenerator::new(self.clone());
        let SpawnResult { macro_pid: pid, .. } = self
            .macro_executor
            .spawn(
                path_to_macro,
                args,
                caused_by,
                Box::new(main_worker_generator),
                None,
                Some(self.uuid.clone()),
                None,
            )
            .await?;
        let entry = TaskEntry {
            pid,
            name: name.to_string(),
            creation_time: chrono::Utc::now().timestamp(),
        };
        self.pid_to_task_entry
            .lock()
            .await
            .insert(pid, entry.clone());
        self.macro_name_to_last_run
            .lock()
            .await
            .insert(name.to_string(), chrono::Utc::now().timestamp());

        Ok(entry)
    }

    async fn kill_macro(&mut self, pid: MacroPID) -> Result<(), Error> {
        self.macro_executor.abort_macro(pid)?;
        Ok(())
    }
}
