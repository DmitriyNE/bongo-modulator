use bongo_modulator::ipc::socket_path;
use tempfile::tempdir;

#[test]
fn socket_uses_env_variable() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("sockenv");
    std::env::set_var("BONGO_SOCKET", &path);
    std::env::remove_var("XDG_RUNTIME_DIR");
    assert_eq!(socket_path(), path);
}

#[test]
fn socket_uses_runtime_dir() {
    let dir = tempdir().unwrap();
    std::env::remove_var("BONGO_SOCKET");
    std::env::set_var("XDG_RUNTIME_DIR", dir.path());
    assert_eq!(socket_path(), dir.path().join("bongo.sock"));
}

#[test]
fn socket_falls_back_to_tempdir() {
    std::env::remove_var("BONGO_SOCKET");
    std::env::remove_var("XDG_RUNTIME_DIR");
    assert_eq!(socket_path(), std::env::temp_dir().join("bongo.sock"));
}
