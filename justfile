[private]
default:
    @just -l

[working-directory: 'lib']
build:
    cargo build

[working-directory: 'lib']
build-release:
    cargo build --release

[working-directory: 'lib']
test:
    cargo test

rust_output_dir := 'lib/target'
debug_dir := rust_output_dir / 'debug'
release_dir := rust_output_dir / 'release'
output_dir := 'lua'
output_name := 'fsharp_tools_rs.so'
output := output_dir / output_name

[linux]
_deploy dir: build
    cp {{dir}}/libfsharp_tools_rs.so {{output}}

[macos]
_deploy dir: build
    cp {{dir}}/libfsharp_tools_rs.dylib {{output}}

[windows]
_deploy dir: build
    copy {{dir}}/fsharp_tools_rs.dll {{output}}

deploy-debug: test build (_deploy debug_dir)

deploy: test build-release (_deploy release_dir)
