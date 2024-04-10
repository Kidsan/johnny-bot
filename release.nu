#!/usr/bin/env -S nu
# build the docker image
nix build .#docker
let built = (docker load -i ./result | lines | last | str replace "Loaded image: " "")
let base = ($built | split row ":" | get 0)
docker tag $built $"($base):latest"
docker push $"($base):latest"
docker push $built
