CREATE KEYSPACE IF NOT EXISTS my_social_network WITH REPLICATION = {
    'class' : 'SimpleStrategy',
    'replication_factor': 1
};