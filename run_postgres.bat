docker stop postgres-tests
docker run --rm --name postgres-tests -e POSTGRES_USER=addresser -e POSTGRES_PASSWORD=addresser -e POSTGRES_DB=addresser -p 5432:5432 -d postgres