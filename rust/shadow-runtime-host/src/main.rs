use std::env;
use std::io::{self, BufRead, Write};
use std::rc::Rc;

use deno_core::anyhow::{anyhow, Context, Result};
use deno_core::url::Url;
use deno_core::v8;
use deno_core::{FsModuleLoader, JsRuntime, PollEventLoopOptions, RuntimeOptions};
use serde::{Deserialize, Serialize};

const RENDER_EXPR: &str = "globalThis.SHADOW_RUNTIME_HOST.render()";
const SESSION_USAGE: &str = "usage: shadow-runtime-host --session <bundle-path>";

fn main() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("build tokio runtime")?;
    runtime.block_on(run())
}

async fn run() -> Result<()> {
    let main_module = resolve_main_module(parse_session_module_path()?)?;
    let mut runtime = load_runtime(&main_module).await?;
    run_session(&mut runtime).await
}

async fn load_runtime(main_module: &Url) -> Result<JsRuntime> {
    let mut runtime = JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(FsModuleLoader)),
        extensions: vec![runtime_nostr_host::init_extension()],
        ..Default::default()
    });

    let module_id = runtime
        .load_main_es_module(main_module)
        .await
        .with_context(|| format!("load module {main_module}"))?;
    let evaluation = runtime.mod_evaluate(module_id);
    runtime
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .context("run deno_core event loop")?;
    evaluation.await.context("evaluate module")?;
    Ok(runtime)
}

async fn run_session(runtime: &mut JsRuntime) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line.context("read session request")?;
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<SessionRequest>(&line) {
            Ok(request) => match handle_session_request(runtime, request).await {
                Ok(payload) => SessionResponse::Ok { payload },
                Err(error) => SessionResponse::Error {
                    message: error.to_string(),
                },
            },
            Err(error) => SessionResponse::Error {
                message: format!("parse session request: {error}"),
            },
        };

        let encoded =
            serde_json::to_string(&response).context("encode runtime session response")?;
        writeln!(stdout, "{encoded}").context("write runtime session response")?;
        stdout.flush().context("flush runtime session response")?;
    }

    Ok(())
}

async fn handle_session_request(
    runtime: &mut JsRuntime,
    request: SessionRequest,
) -> Result<RuntimeDocumentPayload> {
    let expr = match request {
        SessionRequest::Render => String::from(RENDER_EXPR),
        SessionRequest::Dispatch { event } => {
            let event_json =
                serde_json::to_string(&event).context("encode runtime dispatch event")?;
            format!("globalThis.SHADOW_RUNTIME_HOST.dispatch({event_json})")
        }
    };

    let payload_json = execute_string_expr(runtime, &expr, "<session>").await?;
    serde_json::from_str(&payload_json).context("decode runtime document payload")
}

async fn execute_string_expr(
    runtime: &mut JsRuntime,
    expr: &str,
    script_name: &str,
) -> Result<String> {
    let value = runtime
        .execute_script(script_name.to_owned(), expr.to_owned())
        .with_context(|| format!("execute script {script_name}"))?;
    runtime
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .context("run deno_core event loop")?;
    v8_value_to_string(runtime, value)
}

fn v8_value_to_string(runtime: &mut JsRuntime, value: v8::Global<v8::Value>) -> Result<String> {
    deno_core::scope!(scope, runtime);
    let local = v8::Local::new(scope, value);
    local
        .to_string(scope)
        .ok_or_else(|| anyhow!("runtime expression did not evaluate to a string"))
        .map(|value| value.to_rust_string_lossy(scope))
}

fn parse_session_module_path() -> Result<String> {
    let mut args = env::args().skip(1);

    let Some(mode) = args.next() else {
        return Err(anyhow!(SESSION_USAGE));
    };
    if mode != "--session" {
        return Err(anyhow!(SESSION_USAGE));
    }

    let Some(module_path) = args.next() else {
        return Err(anyhow!(SESSION_USAGE));
    };
    if args.next().is_some() {
        return Err(anyhow!(SESSION_USAGE));
    }

    Ok(module_path)
}

fn resolve_main_module(path: String) -> Result<Url> {
    let cwd = env::current_dir().context("get current working directory")?;
    deno_core::resolve_path(&path, &cwd)
        .with_context(|| format!("resolve module path {path} from {}", cwd.display()))
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum SessionRequest {
    Render,
    Dispatch { event: RuntimeDispatchEvent },
}

#[derive(Debug, Deserialize, Serialize)]
struct RuntimeDispatchEvent {
    #[serde(rename = "targetId")]
    target_id: String,
    #[serde(rename = "type")]
    event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selection: Option<RuntimeSelectionEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pointer: Option<RuntimePointerEvent>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RuntimeSelectionEvent {
    #[serde(rename = "start", skip_serializing_if = "Option::is_none")]
    start: Option<u32>,
    #[serde(rename = "end", skip_serializing_if = "Option::is_none")]
    end: Option<u32>,
    #[serde(rename = "direction", skip_serializing_if = "Option::is_none")]
    direction: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RuntimePointerEvent {
    #[serde(rename = "clientX", skip_serializing_if = "Option::is_none")]
    client_x: Option<f32>,
    #[serde(rename = "clientY", skip_serializing_if = "Option::is_none")]
    client_y: Option<f32>,
    #[serde(rename = "isPrimary", skip_serializing_if = "Option::is_none")]
    is_primary: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RuntimeDocumentPayload {
    html: String,
    css: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum SessionResponse {
    Ok { payload: RuntimeDocumentPayload },
    Error { message: String },
}
