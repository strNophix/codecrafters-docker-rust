# codecrafters-docker-rust
My code for CodeCrafter's ["Build Your Own Docker" Challenge](https://codecrafters.io/challenges/docker).

## Requirements
- docker

## Getting started
```sh
alias mydocker='docker build -t mydocker . && docker run --cap-add="SYS_ADMIN" mydocker'
mydocker run debian:latest /bin/sh -c "ls -la /"
```

Note: The `--cap-add="SYS_ADMIN"` flag is required to create
[PID Namespaces](https://man7.org/linux/man-pages/man7/pid_namespaces.7.html)
