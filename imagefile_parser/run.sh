#!/usr/bin/env sh
cd /home/pi
echo -e "100\nq" | /home/pi/linpack > /home/pi/benchmark.log
input="/home/pi/benchmark.log"
while IFS= read -r line
do
  curl --location --request POST '10.0.0.53:8080' \ --header 'Content-Type: text/plain' \ --data-raw "${line}"
done < "$input"