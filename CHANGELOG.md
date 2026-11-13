
- A new file of the directory of a request whose path exists fails the chunk with the failure of the file system, `io`
  with the `errno` `EEXIST` (F-10). Before, it failed with `internal`. The core never makes one file name twice, but
  another process of the same user can create a file in the directory, so the condition is one of the environment.

- `make check` passes on macOS: the helper `last` of `tests/behavior.rs` is used only by the test of a move to another
  file system, which runs on Linux, and it is now inside that test. Before, clippy failed the helper as dead code on
  macOS.

- `Cargo.lock` pins binding at 9de9d03, the commit that every repository of the library family pins (CK-20), and the
  commit of `polyspec-http-media-type` on it; its scaffold writes the same binding packages.
