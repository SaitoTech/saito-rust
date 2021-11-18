# Test Network Guide

---
## Requirements
- Install Docker & Docker compose

## How-to start nodes
Currently, only have 1 node to be enabled (check docker-compose.yml).
Once finishing update on saitocli as expected, I will uncomment that block so that
nodes can interact together.

- start app: `docker-compose up -d --build`
- check app up & running: `docker ps`
- view the saito app logs: `docker logs saito_app`
- in order to stop your container: `docker stop saito_app`