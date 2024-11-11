terraform {
  required_providers {
    digitalocean = {
      source  = "digitalocean/digitalocean"
      version = "~> 2.0"
    }
  }
}

variable "do_token" {}

provider "digitalocean" {
  token = var.do_token
}

resource "digitalocean_droplet" "johnny" {
  image    = "ubuntu-24-10-x64"
  name     = "JohnnyBot"
  region   = "FRA1"
  size     = "s-1vcpu-512mb-10gb"
  ssh_keys = ["97:73:b5:98:a5:e1:11:ef:bf:70:95:32:30:36:d4:a3"]

  connection {
    type        = "ssh"
    user        = "root"
    private_key = file("~/.ssh/id_ed25519")
    host        = self.ipv4_address
  }
  provisioner "remote-exec" {
    inline = [
      "apt-get update",
      "apt-get install -y ca-certificates curl",
      "install -m 0755 -d /etc/apt/keyrings",
      "curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc",
      "chmod a+r /etc/apt/keyrings/docker.asc",
      "echo   \"deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo \"$VERSION_CODENAME\") stable\" | tee /etc/apt/sources.list.d/docker.list > /dev/null ",
      "apt-get update",
      "apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin",
      "groupadd docker",
      "useradd -m johnny",
      "usermod -aG docker johnny",
      "systemctl stop snapd",
      "systemctl mask snapd",
      "systemctl start docker",
      "systemctl enable docker",
    ]
  }
}
