#!/usr/bin/python3
import docker
client = docker.from_env()
containers= client.containers.list()
for container in containers:
    id = container
    print(id.id, id.name)
