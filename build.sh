#!/usr/bin/env bash

mode=

while [[ $# -gt 0 ]]; do
    case $1 in
        -r)
            mode="release"
            shift
            ;;
        *)
            shift
            ;;
    esac
done

buildCommand="cargo build"
if [[ -z $mode ]]; then
    mode="debug"
else
    buildCommand+=" --release"
fi

cd lib || exit

$buildCommand

out_dir="../lua"

rust_output_dir="target/${mode}"

so_path="${rust_output_dir}/libfsharp_tools_rs.so" 
dylib_path="${rust_output_dir}/libfsharp_tools_rs.dylib"
dll_path="${rust_output_dir}/fsharp_tools_rs.dll"
if [ -f $so_path ]; then
    cp -vf $so_path "${out_dir}/fsharp_tools_rs.so"
elif [ -f $dylib_path ]; then
    cp -vf $dylib_path "${out_dir}/fsharp_tools_rs.so"
elif [ -f $dll_path ]; then
    cp -vf $dll_path "${out_dir}/fsharp_tools_rs.so"
else
    echo "no module found"
    exit 1
fi

