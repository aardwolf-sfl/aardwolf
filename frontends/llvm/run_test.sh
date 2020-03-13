SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
LLVM_FRONTEND_PATH=$SCRIPT_DIR/build/bin
VIEW_TOOL_PATH=$SCRIPT_DIR/../../tools
TEMP_FILE=actual.out
FILE_ID_PATTERNS=patterns.sed

for source in tests/*.c
do
    bitcode=$(echo $source | sed 's/.c$/.bc/')
    expected=$(echo $source | sed 's/.c$/.out/')
    aardwold_data=$(echo $source | sed 's/.c$/.bc.aard/')

    if [ -f $expected ]; then
        clang -g -c -emit-llvm -o $bitcode $source
        $LLVM_FRONTEND_PATH/aardwolf_llvm --disable-instrumentation -d tests $bitcode

        python $VIEW_TOOL_PATH/view.py $aardwold_data > $TEMP_FILE
        # Create subsitution patterns for file ids such that original file id given by frontend is replaced with line number in files list
        grep -E '@[0-9]+ = ' $TEMP_FILE | cut -d' ' -f1 | awk '{print "s/" $0 "/@" NR "/"}' > $FILE_ID_PATTERNS
        # Change absolute paths to relative
        sed -i.bak 's,'$(pwd)/',,g' $TEMP_FILE
        # Remove file id from statement id
        sed -i.bak -E 's/#([0-9]+):/#/g' $TEMP_FILE
        # Replace file ids
        sed -i.bak -f $FILE_ID_PATTERNS $TEMP_FILE

        diff --brief $expected $TEMP_FILE  # filename
        diff $expected $TEMP_FILE  # diff
    fi
done

rm -f $TEMP_FILE $FILE_ID_PATTERNS $TEMP_FILE.bak
