SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
LLVM_PASSES_PATH=$SCRIPT_DIR/build/lib
VIEW_TOOL_PATH=$SCRIPT_DIR/../../tools
TEMP_FILE=actual.out

for source in tests/*.c
do
    bitcode=$(echo $source | sed 's/.c$/.bc/')
    expected=$(echo $source | sed 's/.c$/.out/')
    aardwold_data="tests/aardwolf.$(echo $bitcode | sed 's,tests/,,').data"

    if [ -f $expected ]; then
        clang -g -c -emit-llvm -o $bitcode $source
        opt -load $LLVM_PASSES_PATH/libLLVMStatementDetection.so -load $LLVM_PASSES_PATH/libLLVMStaticData.so -aard-static-data $bitcode > /dev/null
        mv *.data tests/
        python $VIEW_TOOL_PATH/view.py $aardwold_data | sed 's,'$(pwd)/',,g' > $TEMP_FILE
        diff --brief $expected $TEMP_FILE  # filename
        diff $expected $TEMP_FILE  # diff
    fi
done

rm -f $TEMP_FILE
