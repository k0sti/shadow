import { core } from "ext:core/mod.js";

function installShadowRuntimeOs() {
  const shadow = globalThis.Shadow ?? {};
  const os = shadow.os ?? {};
  const nostr = {
    listKind1(query = {}) {
      return core.ops.op_runtime_nostr_list_kind1(query);
    },
    publishKind1(request = {}) {
      return core.ops.op_runtime_nostr_publish_kind1(request);
    },
  };

  globalThis.Shadow = {
    ...shadow,
    os: {
      ...os,
      nostr,
    },
  };
}

installShadowRuntimeOs();
