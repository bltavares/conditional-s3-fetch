#/!/bin/bash

# Start minio server
minio server /tmp --console-address ":9001" &

sleep 5
mc alias set local http://localhost:9000 $MINIO_ROOT_USER $MINIO_ROOT_PASSWORD
mc mb local/example-bucket
mc mirror /data/example local/example-bucket

wait
