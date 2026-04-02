use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use deno_core::FsModuleLoader;
use deno_core::JsRuntime;
use deno_core::PollEventLoopOptions;
use deno_core::RuntimeOptions;
use deno_core::anyhow::{Context, Result, anyhow};
use deno_core::extension;
use deno_core::op2;
use deno_error::JsErrorBox;

const DEFAULT_RESULT_EXPR: &str = "globalThis.RUNTIME_SMOKE_RESULT";

#[op2]
#[string]
async fn op_runtime_message(#[string] prefix: String) -> Result<String, JsErrorBox> {
    tokio::time::sleep(Duration::from_millis(1)).await;
    Ok(format!("{prefix} FROM HOST OP"))
}

extension!(runtime_smoke_extension, ops = [op_runtime_message]);

fn main() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("build tokio runtime")?;
    runtime.block_on(run())
}

async fn run() -> Result<()> {
    let options = parse_options()?;
    let main_module = resolve_main_module(options.module_path)?;
    let mut runtime = JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(FsModuleLoader)),
        extensions: vec![runtime_smoke_extension::init()],
        ..Default::default()
    });

    let module_id = runtime
        .load_main_es_module(&main_module)
        .await
        .with_context(|| format!("load module {main_module}"))?;
    let evaluation = runtime.mod_evaluate(module_id);
    runtime
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .context("run deno_core event loop")?;
    evaluation.await.context("evaluate module")?;

    let value = runtime
        .execute_script("<result>", options.result_expr)
        .context("read runtime smoke result")?;

    deno_core::scope!(scope, runtime);
    let local = deno_core::v8::Local::new(scope, value);
    let value = local
        .to_string(scope)
        .ok_or_else(|| anyhow!("runtime smoke result was not a string"))?
        .to_rust_string_lossy(scope);

    println!(
        "deno_core host-op ok: target={} module={} result={value}",
        std::env::consts::ARCH,
        main_module
    );
    Ok(())
}

struct Options {
    module_path: Option<String>,
    result_expr: String,
}

fn parse_options() -> Result<Options> {
    let mut args = env::args().skip(1);
    let mut module_path = None;
    let mut result_expr = String::from(DEFAULT_RESULT_EXPR);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--result-expr" => {
                result_expr = args
                    .next()
                    .ok_or_else(|| anyhow!("missing value for --result-expr"))?;
            }
            _ if module_path.is_none() => {
                module_path = Some(arg);
            }
            _ => {
                return Err(anyhow!("unknown argument: {arg}"));
            }
        }
    }

    Ok(Options {
        module_path,
        result_expr,
    })
}

fn resolve_main_module(module_path: Option<String>) -> Result<deno_core::url::Url> {
    if let Some(path) = module_path {
        return resolve_from_cwd(path);
    }

    for candidate in bundled_module_candidates()? {
        if candidate.is_file() {
            return deno_core::url::Url::from_file_path(&candidate)
                .map_err(|_| anyhow!("resolve bundled module path {}", candidate.display()));
        }
    }

    Err(anyhow!(
        "could not find bundled module; pass a path explicitly or run from the package output"
    ))
}

fn resolve_from_cwd(path: String) -> Result<deno_core::url::Url> {
    let cwd = env::current_dir().context("get current working directory")?;
    deno_core::resolve_path(&path, &cwd)
        .with_context(|| format!("resolve module path {path} from {}", cwd.display()))
}

fn bundled_module_candidates() -> Result<Vec<PathBuf>> {
    let current_exe = env::current_exe().context("resolve current executable")?;
    let bundle_from_exe = current_exe
        .parent()
        .and_then(|bin_dir| bin_dir.parent())
        .map(|prefix| prefix.join("lib/deno-core-smoke/modules/main.js"));
    let manifest_bundle = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("modules/main.js");

    Ok(bundle_from_exe
        .into_iter()
        .chain(std::iter::once(manifest_bundle))
        .collect())
}
