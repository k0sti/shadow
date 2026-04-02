use std::sync::Once;

fn init_v8() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

fn main() {
    init_v8();

    let isolate = &mut v8::Isolate::new(Default::default());
    let mut handle_scope = std::pin::pin!(v8::HandleScope::new(isolate));
    let handle_scope = &mut handle_scope.init();
    let context = v8::Context::new(handle_scope, Default::default());
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    let source = v8::String::new(scope, "'hello from v8'.toUpperCase()").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let value = script.run(scope).unwrap();
    let value = value.to_string(scope).unwrap();

    println!(
        "rusty_v8 ok: target={} result={}",
        std::env::consts::ARCH,
        value.to_rust_string_lossy(scope)
    );
}
