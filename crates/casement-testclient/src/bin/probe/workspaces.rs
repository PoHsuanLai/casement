//! `casement-probe workspaces`: binds `ext_workspace_manager_v1`, waits (bounded) for the first
//! `done`, and prints one JSON line per workspace in announce order:
//! `{"id": "..."|null, "name": "...", "group": 0|null, "active": true|false}`.

use crate::Failure;
use casement_testclient::TestClient;
use casement_testclient::manage::workspace::{Shown, WorkspaceSeen, WorkspaceWatch};
use serde_json::{Value, json};
use std::time::Duration;

/// How many turns the probe waits for the first `done`.
const TURNS: u32 = 250;
/// How long one turn lasts (so the wait is about five seconds).
const TURN: Duration = Duration::from_millis(20);

/// Every workspace the compositor announces, one line each.
pub(crate) fn run() -> Result<(), Failure> {
    let client = TestClient::connect_env()?;
    let mut watch = WorkspaceWatch::start(client.globals())?;
    for _ in 0..TURNS {
        watch.pump()?;
        if watch.dones() > 0 {
            for workspace in watch.workspaces() {
                println!("{}", line(&workspace));
            }
            return Ok(());
        }
        std::thread::sleep(TURN);
    }
    Err(Failure::Probe(
        "the compositor sent no ext_workspace_manager_v1 done within five seconds".to_owned(),
    ))
}

/// One workspace as its JSON line.
fn line(workspace: &WorkspaceSeen) -> Value {
    json!({
        "id": workspace.id,
        "name": workspace.name,
        "group": workspace.group,
        "active": workspace.shown == Shown::Yes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_workspace_is_one_json_object() {
        let cases = [
            (
                WorkspaceSeen {
                    id: Some("abc".to_owned()),
                    name: "Main".to_owned(),
                    shown: Shown::Yes,
                    group: Some(0),
                    ..WorkspaceSeen::default()
                },
                json!({ "id": "abc", "name": "Main", "group": 0, "active": true }),
            ),
            (
                WorkspaceSeen::default(),
                json!({ "id": null, "name": "", "group": null, "active": false }),
            ),
        ];
        for (workspace, want) in cases {
            assert_eq!(line(&workspace), want);
        }
    }
}
