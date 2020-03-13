# Write Aardwolf header
[[ ! -z "$AARDWOLF_EXTERNAL" ]] && $AARDWOLF_EXTERNAL

rm -f *.diff

for input in *.in
do
    output="$(basename $input .in).out"
    diff_file="$(basename $input .in).diff"

    # Indicate new test case
    [[ ! -z "$AARDWOLF_EXTERNAL" ]] && $AARDWOLF_EXTERNAL "$input"

    cat $input | xargs ../bin/sorted | diff - $output > $diff_file

    if [ "$?" = "0" ]
    then
        rm $diff_file
        echo "PASS: $input"
    else
        echo "FAIL: $input"
    fi
done
