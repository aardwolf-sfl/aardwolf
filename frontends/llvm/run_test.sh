SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
LLVM_FRONTEND_PATH=$SCRIPT_DIR/build/bin
VIEW_TOOL_PATH=$SCRIPT_DIR/../../tools
TEMP_FILE=actual.out

for source in tests/*.c
do
    bitcode=$(echo $source | sed 's/.c$/.bc/')
    expected=$(echo $source | sed 's/.c$/.out/')
    aardwold_data=$(echo $source | sed 's/.c$/.bc.aard/')

    if [ -f $expected ]; then
        clang -g -c -emit-llvm -o $bitcode $source
        $LLVM_FRONTEND_PATH/aardwolf_llvm --disable-instrumentation -o tests $bitcode
        python $VIEW_TOOL_PATH/view.py $aardwold_data | sed 's,'$(pwd)/',,g' > $TEMP_FILE
        diff --brief $expected $TEMP_FILE  # filename
        diff $expected $TEMP_FILE  # diff
    fi
done

rm -f $TEMP_FILE
