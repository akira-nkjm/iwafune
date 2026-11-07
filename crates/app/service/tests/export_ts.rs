//! API の型を `web/src/api/gen/` に TS として書き出す。CI はこのあと `git diff --exit-code web/src/api/gen` で差分が無いことを確かめる。

use std::path::Path;

#[test]
fn export_ts() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../web/src/api/gen");
    let files = fune_service::dto::export_ts(&dir).expect("TS を書き出せること");
    assert!(files.iter().any(|f| f.ends_with("index.ts")));
}
