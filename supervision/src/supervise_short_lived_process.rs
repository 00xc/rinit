use anyhow::Result;
use rinit_ipc::{
    request_error::RequestError,
    Reply,
    Request,
};
use rinit_service::types::Oneshot;

use crate::{
    async_connection::AsyncConnection,
    run_short_lived_script,
    signal_wait::signal_wait_fun,
};

pub async fn supervise_short_lived_process(
    phase: &str,
    service: &str,
) -> Result<()> {
    let start = match phase {
        "start" => true,
        "stop" => false,
        _ => todo!(),
    };
    let oneshot: Oneshot = serde_json::from_str(service)?;
    let mut conn = AsyncConnection::new_host_address().await?;
    let request = Request::ServiceIsUp(
        oneshot.name,
        if start {
            run_short_lived_script(&oneshot.start, signal_wait_fun()).await?
        } else {
            if let Some(stop_script) = oneshot.stop {
                run_short_lived_script(&stop_script, signal_wait_fun()).await?;
            }
            false
        },
    );

    // TODO: log this
    conn.send_request(request).await.unwrap();
    let reply: Result<Reply, RequestError> = serde_json::from_str(&conn.recv().await?)?;
    reply?;

    Ok(())
}
