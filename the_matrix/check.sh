#!/bin/bash
# By u/superdupermicrochip

output=`cargo check -p $1 --color always 2>&1`
res=$?

if [ "0" -eq "${res}" ];
then
    echo -e "${output}"
    exit 0
fi

# Sed line : https://stackoverflow.com/a/51141872
output_nocolor=`echo -e "${output}" | sed 's/\x1B\[[0-9;]\{1,\}[A-Za-z]//g'`

first=`echo -e "${output_nocolor}" | egrep -nm 1 "^error" | egrep -o "^[0-9]*"`
let skip=${first}-1

head_num=`echo -e "${output_nocolor}" | sed 1,${skip}d | egrep -nm 1 "^$" | egrep -o "^[0-9]*"`

echo -e "${output}" | sed 1,${skip}d | head -n ${head_num}
echo -e "${output}" | tail -n 1
exit ${res}
