# Deployment scaffold

This directory currently contains only a local Docker Compose scaffold for PostgreSQL. Server, Google Drive adapter, reverse proxy, and production deployment wiring are intentionally deferred to later phases.

Keep local private configuration outside the repository.

## Local validation

Validate the Compose file without starting services:

~~~bash
docker compose -f deploy/docker-compose.yml config
~~~

Do not commit `.env` files, OAuth material, production credentials, generated logs, or local data directories.
