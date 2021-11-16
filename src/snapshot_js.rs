use deno_core::Snapshot;

pub static CLI_SNAPSHOT: &[u8] =
    include_bytes!("../snapshot-bin/SNAPSHOT_CLI.bin");

pub fn three_em_isolate() -> Snapshot {
    let data = CLI_SNAPSHOT;
    Snapshot::Static(data)
}
