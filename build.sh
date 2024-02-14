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
cp -vf "target/${mode}/libfsharp_tools_rs.so" ".."
