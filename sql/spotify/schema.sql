create table
    if not exists kv (
        id integer primary key autoincrement,
        key text not null,
        value json not null,
        ts timestamp default current_timestamp
    );
