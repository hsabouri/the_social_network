#!/bin/sh

# PostgreSQL
docker run -it --rm --network host -v $(pwd)/migration:/usr/migration/ postgres psql postgresql://user:password@localhost:5432/my_social_network_db --file=/usr/migration/init_dev/users.sql
docker run -it --rm --network host -v $(pwd)/migration:/usr/migration/ postgres psql postgresql://user:password@localhost:5432/my_social_network_db --file=/usr/migration/init_dev/friendships.sql

# ScyllaDB
docker exec -it scylladb cqlsh -f /usr/migration/init_dev/keyspace.cql
# docker exec -it scylladb cqlsh -f /usr/migration/init_dev/functions.cql -k my_social_network
docker exec -it scylladb cqlsh -f /usr/migration/init_dev/messages.cql -k my_social_network