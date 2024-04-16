#!/usr/bin/env -S nu
# build the docker image
nix build .#docker
let built = (docker load -i ./result | lines | last | str replace "Loaded image: " "")
let base = ($built | split row ":" | get 0)
docker tag $built $"($base):latest"
docker push $"($base):latest"
docker push $built
ssh newjohnnybot $"docker pull ($base):latest"

let proceed = (input --numchar 1 "Proceed? [y/n]") | str downcase
if $proceed == "y" {
  ssh newjohnnybot "cd bot && docker rm -f bot-johnny-1 && docker compose up -d"
}
