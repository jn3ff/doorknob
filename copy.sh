#!/bin/bash

./build.sh

echo "copying bin to rpi"
scp target/aarch64-unknown-linux-gnu/release/$PROJECT_NAME $PI_USER@$PI_IP:~/$PROJECT_NAME
ssh $PI_USER@$PI_IP "chmod +x ~/$PROJECT_NAME"

# deploy on pi as there is some runtime setup

echo "done"
