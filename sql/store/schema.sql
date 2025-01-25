create table
    if not exists kv (
        key text primary key not null,
        value json not null
    );
