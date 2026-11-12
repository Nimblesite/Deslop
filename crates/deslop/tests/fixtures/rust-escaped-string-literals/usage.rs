pub(crate) const USAGE: &str =
    "usage: osprey <file-or-project> [--check | --ast | --llvm | --compile | --run | \
--symbols | --signature | --source-graph | --list-tests | --doctests | --deps] [--quiet] [--no-warnings | --warnings-as-errors] [--debug] [--profile] [--flavor default|ml|python] \
[--memory=gc|arc|none] [--target=native|wasm32|ios|ios-sim|macos|android-arm64|android-x64|linux-x64|linux-arm64|windows-x64|python] [--entry-only] [-o <out>] \
[--sandbox | --no-http | --no-websocket | --no-fs | --no-ffi]\n\
       osprey build [project] [--quiet] [--no-warnings | --warnings-as-errors] [--debug] [--memory=gc|arc|none] \
[--target=native|wasm32|ios|ios-sim|macos|android-arm64|android-x64|linux-x64|linux-arm64|windows-x64|python] [-o <out>]\n\
       osprey test [path] [--filter <name>] [--quiet] [--coverage] \
[--coverage-json <path>] [--memory=gc|arc|none]\n\
       osprey fmt [--check | --stdout] [--flavor default|ml|python] <path...>\n\
       osprey hunt <file> [--max-time <seconds>] [--max-memory <mb>] [--input-bytes <n>] [--out <dir>] [--json] [--no-replay] [--quiet]\n\
       osprey --hover <name>\n\
       osprey --docs [<file-or-project> | --source <file-or-project>] --docs-dir <dir> \
[--docs-format markdown|html] [--docs-theme osprey|midnight|paper] \
[--docs-page <markdown-file-or-directory>]... [--docs-css <stylesheet>]...\n\
       osprey lsp\n\
Debug options: [--debug-info=dwarf|none] [--debug-opt=none] [--debug-memory=off] \
[--debug-out <path>] [--debug-preserve-ir] [--debug-preserve-symbols]";
