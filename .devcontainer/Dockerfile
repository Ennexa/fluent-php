FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

RUN apt update && \
    apt install -y software-properties-common ca-certificates lsb-release apt-transport-https && \
    sh -c 'echo "deb https://packages.sury.org/php/ $(lsb_release -sc) main" > /etc/apt/sources.list.d/php.list'  && \
    wget -qO - https://packages.sury.org/php/apt.gpg | apt-key add - && \
    apt update && \
    apt install -y php8.1 php8.2