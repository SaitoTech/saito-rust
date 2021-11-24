# Local Test Network Guide

---
## Requirements
- Install Docker & Docker compose

## How-to start nodes

- start nodes: `CONF_FILE=config_file_path W_PWD=password_string docker-compose -f docker-compose.test.yml up  -d --build`
- check nodes up & running: `docker ps`
- view the saito nodes logs:
  ```
  docker logs saito_app        # view logs on node 1
  docker logs saito_app_node1  # view logs on node 2
  ```
- Stop your nodes: `docker stop saito_app saito_app_node1`