#/bin/sh
echo */ | sed "s/\//\n/g" | sed "s/^ //g" | while read line
do
	echo "$line"
	git ls-files | grep "$1" | grep "^$line/" | xargs wc -l
	echo "**************************************************"
done
